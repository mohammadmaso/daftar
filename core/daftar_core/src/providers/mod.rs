//! Bring-your-own-key AI providers (§9). One trait, three wire formats, plus a scripted mock.

mod anthropic;
mod gemini;
pub mod mock;
mod openai;
mod probe;
mod retry;
mod sse;

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ledger::Usage;

pub use anthropic::Anthropic;
pub use gemini::Gemini;
pub use mock::MockProvider;
pub use openai::OpenAiCompatible;
pub use probe::{ProbeResult, probe};
pub use retry::Retrying;

// ─────────────────────────── configuration ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenaiCompatible,
    Anthropic,
    Gemini,
    /// Replays recorded responses; used by tests and `daftar eval`.
    Mock,
}

/// Stored in `.daftar/config.json` — never contains secrets (§9).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub extra_headers: BTreeMap<String, String>,
    #[serde(default = "default_timeout")]
    pub timeout_s: u64,
}

fn default_timeout() -> u64 {
    60
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Router,
    Ingest,
    Chat,
    Voice,
    Vision,
    Reflect,
    Lint,
    Stt,
    Tts,
    Embedding,
}

impl Role {
    pub const ALL: [Role; 10] = [
        Role::Router,
        Role::Ingest,
        Role::Chat,
        Role::Voice,
        Role::Vision,
        Role::Reflect,
        Role::Lint,
        Role::Stt,
        Role::Tts,
        Role::Embedding,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Role::Router => "router",
            Role::Ingest => "ingest",
            Role::Chat => "chat",
            Role::Voice => "voice",
            Role::Vision => "vision",
            Role::Reflect => "reflect",
            Role::Lint => "lint",
            Role::Stt => "stt",
            Role::Tts => "tts",
            Role::Embedding => "embedding",
        }
    }

    /// Roles that fall back to another when not configured (keeps setup short).
    pub fn fallback(self) -> Option<Role> {
        match self {
            Role::Router | Role::Reflect | Role::Lint | Role::Voice => Some(Role::Chat),
            Role::Ingest => Some(Role::Chat),
            Role::Vision => Some(Role::Chat),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleConfig {
    pub role: Role,
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub params: serde_json::Map<String, Value>,
}

/// User-editable per-model prices, USD per million tokens.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Price {
    pub input: f64,
    pub output: f64,
    #[serde(default)]
    pub cached_input: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
    #[serde(default)]
    pub roles: Vec<RoleConfig>,
    #[serde(default)]
    pub prices: BTreeMap<String, Price>,
}

impl AiConfig {
    pub fn role(&self, role: Role) -> Option<&RoleConfig> {
        let mut r = Some(role);
        while let Some(x) = r {
            if let Some(c) = self.roles.iter().find(|c| c.role == x) {
                return Some(c);
            }
            r = x.fallback();
        }
        None
    }

    pub fn provider(&self, id: &str) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.id == id)
    }

    /// Adds or replaces a provider. An empty id gets a fresh one. Returns the id.
    pub fn upsert_provider(&mut self, mut p: ProviderConfig) -> String {
        if p.id.trim().is_empty() {
            p.id = format!("p-{}", crate::ids::new_id().to_string().to_lowercase());
        }
        let id = p.id.clone();
        match self.providers.iter_mut().find(|x| x.id == id) {
            Some(x) => *x = p,
            None => self.providers.push(p),
        }
        id
    }

    /// Removes a provider and every role that used it (those roles fall back again).
    pub fn remove_provider(&mut self, id: &str) {
        self.providers.retain(|p| p.id != id);
        self.roles.retain(|r| r.provider != id);
    }

    /// Points `role` at a provider and model, keeping parameters when only the model changes.
    pub fn set_role(&mut self, role: Role, provider: &str, model: &str) {
        match self.roles.iter_mut().find(|r| r.role == role) {
            Some(r) => {
                if r.provider != provider {
                    r.params.clear();
                }
                r.provider = provider.to_owned();
                r.model = model.trim().to_owned();
            }
            None => self.roles.push(RoleConfig {
                role,
                provider: provider.to_owned(),
                model: model.trim().to_owned(),
                params: Default::default(),
            }),
        }
    }

    /// Removes the explicit setting for `role`; it falls back to another role if it can.
    pub fn clear_role(&mut self, role: Role) {
        self.roles.retain(|r| r.role != role);
    }

    /// The role whose setting `role` actually uses, if it is not set itself.
    pub fn inherited_from(&self, role: Role) -> Option<Role> {
        let used = self.role(role)?.role;
        (used != role).then_some(used)
    }

    pub fn cost(&self, model: &str, u: &Usage) -> Option<f64> {
        let p = self.prices.get(model)?;
        let cached = p.cached_input.unwrap_or(p.input);
        let fresh = u.input_tokens.saturating_sub(u.cached_input_tokens) as f64;
        Some(
            (fresh * p.input
                + u.cached_input_tokens as f64 * cached
                + u.output_tokens as f64 * p.output)
                / 1e6,
        )
    }
}

// ─────────────────────────── messages ───────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Part {
    Text {
        text: String,
    },
    /// Base64 image data.
    Image {
        media_type: String,
        data: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MsgRole {
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: MsgRole,
    pub parts: Vec<Part>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    /// For `Tool` messages: the call being answered.
    #[serde(default)]
    pub tool_call_id: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: MsgRole::User,
            parts: vec![Part::Text { text: text.into() }],
            tool_calls: vec![],
            tool_call_id: None,
            tool_name: None,
        }
    }
    pub fn assistant(text: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        let text = text.into();
        let parts = if text.is_empty() {
            vec![]
        } else {
            vec![Part::Text { text }]
        };
        Self {
            role: MsgRole::Assistant,
            parts,
            tool_calls,
            tool_call_id: None,
            tool_name: None,
        }
    }
    pub fn tool(call: &ToolCall, result: impl Into<String>) -> Self {
        Self {
            role: MsgRole::Tool,
            parts: vec![Part::Text {
                text: result.into(),
            }],
            tool_calls: vec![],
            tool_call_id: Some(call.id.clone()),
            tool_name: Some(call.name.clone()),
        }
    }
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| match p {
                Part::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// JSON Schema of the arguments object.
    pub parameters: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub system: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub tools: Vec<ToolSpec>,
    pub max_tokens: u32,
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Ask for a single JSON object as the reply.
    #[serde(default)]
    pub json: bool,
    #[serde(default)]
    pub params: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    pub stop: StopReason,
}

// ─────────────────────────── errors ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderErrorKind {
    Auth,
    RateLimited,
    Network,
    Timeout,
    BadRequest,
    Server,
    NotSupported,
    NotConfigured,
}

/// Provider failure in human language (§9: no raw stack traces).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{message}")]
pub struct ProviderError {
    pub kind: ProviderErrorKind,
    pub message: String,
    pub retry_after_s: Option<u64>,
}

impl ProviderError {
    pub fn new(kind: ProviderErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retry_after_s: None,
        }
    }

    /// Whether retrying later can succeed (network blips, rate limits, 5xx).
    pub fn transient(&self) -> bool {
        matches!(
            self.kind,
            ProviderErrorKind::RateLimited
                | ProviderErrorKind::Network
                | ProviderErrorKind::Timeout
                | ProviderErrorKind::Server
        )
    }

    pub(crate) fn from_status(
        provider: &str,
        status: u16,
        body: &str,
        retry_after: Option<u64>,
    ) -> Self {
        let detail = extract_message(body);
        let (kind, msg) = match status {
            401 | 403 => (
                ProviderErrorKind::Auth,
                format!("{provider} rejected the API key."),
            ),
            429 => (
                ProviderErrorKind::RateLimited,
                format!("{provider} is rate-limiting requests; will retry."),
            ),
            404 => (
                ProviderErrorKind::BadRequest,
                format!("{provider} doesn't know this model or endpoint."),
            ),
            400 | 413 | 422 => (
                ProviderErrorKind::BadRequest,
                format!("{provider} refused the request."),
            ),
            500..=599 => (
                ProviderErrorKind::Server,
                format!("{provider} had a server error; will retry."),
            ),
            _ => (
                ProviderErrorKind::Server,
                format!("{provider} answered with status {status}."),
            ),
        };
        let message = match detail {
            Some(d) if !d.is_empty() => {
                format!("{msg} ({})", d.chars().take(200).collect::<String>())
            }
            _ => msg,
        };
        Self {
            kind,
            message,
            retry_after_s: retry_after,
        }
    }

    pub(crate) fn from_reqwest(provider: &str, e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::new(
                ProviderErrorKind::Timeout,
                format!("{provider} took too long to answer."),
            )
        } else if e.is_connect() || e.is_request() {
            Self::new(
                ProviderErrorKind::Network,
                format!("Couldn't reach {provider}."),
            )
        } else if e.is_decode() || e.is_body() {
            Self::new(
                ProviderErrorKind::Server,
                format!("{provider} sent a response that couldn't be read."),
            )
        } else {
            Self::new(
                ProviderErrorKind::Network,
                format!("Connection to {provider} failed."),
            )
        }
    }
}

fn extract_message(body: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    v.pointer("/error/message")
        .or_else(|| v.pointer("/message"))
        .or_else(|| v.pointer("/error"))
        .and_then(|m| m.as_str().map(str::to_owned))
}

pub type ProviderResult<T> = std::result::Result<T, ProviderError>;

/// Streaming callback: receives text deltas as they arrive.
pub type OnDelta<'a> = Option<&'a (dyn Fn(&str) + Send + Sync)>;

#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn chat(&self, req: &ChatRequest, on_delta: OnDelta<'_>) -> ProviderResult<ChatResponse>;
    async fn transcribe(
        &self,
        _model: &str,
        _audio: Vec<u8>,
        _file_name: &str,
        _language: Option<&str>,
    ) -> ProviderResult<String> {
        Err(ProviderError::new(
            ProviderErrorKind::NotSupported,
            format!("{} has no speech-to-text endpoint.", self.name()),
        ))
    }
    async fn speech(&self, _model: &str, _voice: &str, _text: &str) -> ProviderResult<Vec<u8>> {
        Err(ProviderError::new(
            ProviderErrorKind::NotSupported,
            format!("{} has no text-to-speech endpoint.", self.name()),
        ))
    }
    async fn embed(&self, _model: &str, _inputs: &[String]) -> ProviderResult<Vec<Vec<f32>>> {
        Err(ProviderError::new(
            ProviderErrorKind::NotSupported,
            format!("{} has no embeddings endpoint.", self.name()),
        ))
    }
    async fn list_models(&self) -> ProviderResult<Vec<String>> {
        Ok(vec![])
    }
}

pub type DynProvider = Arc<dyn LlmProvider>;

/// Builds a provider from its config and secret. `mock_script` is used for `ProviderKind::Mock`.
pub fn build(config: &ProviderConfig, api_key: Option<String>) -> ProviderResult<DynProvider> {
    let key = api_key.unwrap_or_default();
    let timeout = std::time::Duration::from_secs(config.timeout_s.max(5));
    Ok(match config.kind {
        ProviderKind::OpenaiCompatible => {
            retry::wrap(Arc::new(OpenAiCompatible::new(config, key, timeout)))
        }
        ProviderKind::Anthropic => retry::wrap(Arc::new(Anthropic::new(config, key, timeout))),
        ProviderKind::Gemini => retry::wrap(Arc::new(Gemini::new(config, key, timeout))),
        ProviderKind::Mock => Arc::new(
            MockProvider::from_file(&config.base_url)
                .map_err(|e| ProviderError::new(ProviderErrorKind::NotConfigured, e.to_string()))?,
        ),
    })
}

/// Capability hints for warnings in role settings (§9). Heuristic by model name; never blocks.
pub fn capability_warning(role: Role, model: &str) -> Option<&'static str> {
    let m = model.to_lowercase();
    match role {
        Role::Vision
            if [
                "whisper",
                "embedding",
                "tts",
                "gpt-3.5",
                "deepseek-r1",
                "llama-3.1",
                "mixtral",
            ]
            .iter()
            .any(|x| m.contains(x)) =>
        {
            Some("This model probably can't read images.")
        }
        Role::Stt
            if !["whisper", "transcribe", "stt", "speech", "scribe", "nova"]
                .iter()
                .any(|x| m.contains(x)) =>
        {
            Some("This doesn't look like a speech-to-text model.")
        }
        Role::Tts
            if !["tts", "speech", "voice", "sonic", "aura"]
                .iter()
                .any(|x| m.contains(x)) =>
        {
            Some("This doesn't look like a text-to-speech model.")
        }
        Role::Embedding if !m.contains("embed") => {
            Some("This doesn't look like an embedding model.")
        }
        Role::Router | Role::Ingest | Role::Chat | Role::Voice | Role::Reflect | Role::Lint
            if ["whisper", "embed", "tts-"].iter().any(|x| m.contains(x)) =>
        {
            Some("This model can't hold a conversation.")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_fall_back_to_chat() {
        let cfg = AiConfig {
            providers: vec![],
            roles: vec![RoleConfig {
                role: Role::Chat,
                provider: "p1".into(),
                model: "m".into(),
                params: Default::default(),
            }],
            prices: Default::default(),
        };
        assert_eq!(cfg.role(Role::Router).unwrap().role, Role::Chat);
        assert!(cfg.role(Role::Stt).is_none());
    }

    #[test]
    fn editing_providers_and_roles() {
        let mut cfg = AiConfig::default();
        let id = cfg.upsert_provider(ProviderConfig {
            id: String::new(),
            name: "OpenRouter".into(),
            kind: ProviderKind::OpenaiCompatible,
            base_url: "https://openrouter.ai/api/v1".into(),
            extra_headers: Default::default(),
            timeout_s: 60,
        });
        assert!(id.starts_with("p-"));
        cfg.set_role(Role::Chat, &id, " gpt-5-mini ");
        assert_eq!(cfg.role(Role::Chat).unwrap().model, "gpt-5-mini");
        assert_eq!(cfg.inherited_from(Role::Router), Some(Role::Chat));
        assert_eq!(cfg.inherited_from(Role::Chat), None);

        let mut renamed = cfg.provider(&id).unwrap().clone();
        renamed.name = "My OpenRouter".into();
        assert_eq!(cfg.upsert_provider(renamed), id);
        assert_eq!(cfg.providers.len(), 1);
        assert_eq!(cfg.providers[0].name, "My OpenRouter");

        cfg.clear_role(Role::Chat);
        assert!(cfg.role(Role::Router).is_none());
        cfg.set_role(Role::Stt, &id, "whisper-1");
        cfg.remove_provider(&id);
        assert!(cfg.providers.is_empty() && cfg.roles.is_empty());
    }

    #[test]
    fn cost_uses_cached_price() {
        let mut cfg = AiConfig::default();
        cfg.prices.insert(
            "m".into(),
            Price {
                input: 3.0,
                output: 15.0,
                cached_input: Some(0.3),
            },
        );
        let u = Usage {
            input_tokens: 1_000_000,
            output_tokens: 100_000,
            cached_input_tokens: 500_000,
            cost_usd: None,
        };
        let c = cfg.cost("m", &u).unwrap();
        assert!((c - (1.5 + 0.15 + 1.5)).abs() < 1e-9, "{c}");
    }

    #[test]
    fn errors_are_human() {
        let e = ProviderError::from_status(
            "OpenRouter",
            401,
            r#"{"error":{"message":"No auth credentials found"}}"#,
            None,
        );
        assert_eq!(e.kind, ProviderErrorKind::Auth);
        assert_eq!(
            e.message,
            "OpenRouter rejected the API key. (No auth credentials found)"
        );
        assert!(ProviderError::from_status("X", 503, "", None).transient());
        assert!(!e.transient());
    }

    #[test]
    fn capability_warnings() {
        assert!(capability_warning(Role::Vision, "whisper-large-v3").is_some());
        assert!(capability_warning(Role::Stt, "whisper-large-v3").is_none());
        assert!(capability_warning(Role::Chat, "claude-sonnet-5").is_none());
    }
}
