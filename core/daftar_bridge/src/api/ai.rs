//! AI providers, model roles and the job runner for the Flutter app (§9). Provider configs live in
//! the shared `.daftar/config.json`; API keys stay in platform secure storage and are passed in for
//! each call only.

use std::collections::{BTreeMap, HashMap};

use daftar_core::agent::Cancel;
use daftar_core::providers::{self, ProviderConfig, ProviderKind, Role};
use daftar_core::queue::{JobKind, JobState};

use super::library::LibraryHandle;

pub enum ProviderKindDto {
    OpenaiCompatible,
    Anthropic,
    Gemini,
}

pub enum ModelRole {
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

pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

pub struct AiProvider {
    /// Empty when creating a new provider.
    pub id: String,
    pub name: String,
    pub kind: ProviderKindDto,
    /// Empty means the provider's public endpoint.
    pub base_url: String,
    pub headers: Vec<HttpHeader>,
    pub timeout_s: u32,
}

pub struct RoleSetting {
    pub role: ModelRole,
    /// Provider and model this role uses, possibly through a fallback role.
    pub provider_id: Option<String>,
    pub model: Option<String>,
    /// Set when the role is not configured itself and borrows another role's setting.
    pub inherited_from: Option<ModelRole>,
    /// Heuristic capability mismatch, in English; the app shows its own localized sentence.
    pub warning: Option<CapabilityWarning>,
}

pub enum CapabilityWarning {
    NoVision,
    NotSpeechToText,
    NotTextToSpeech,
    NotEmbedding,
    NotConversational,
}

pub struct AiSettings {
    pub providers: Vec<AiProvider>,
    pub roles: Vec<RoleSetting>,
}

/// An API key for one provider, read from secure storage by the app.
pub struct ApiKey {
    pub provider_id: String,
    pub key: String,
}

pub struct ProbeOutcome {
    pub ok: bool,
    pub latency_ms: u32,
    /// Short reply on success ("OK", "no speech (as expected)"), one human sentence on failure.
    pub detail: String,
}

pub enum JobKindDto {
    Transcribe,
    Describe,
    Ingest,
    Other,
}

pub enum JobStateDto {
    Queued,
    Running,
    Done,
    Failed,
}

pub struct JobOutcome {
    pub kind: JobKindDto,
    pub raw_id: Option<String>,
    pub state: JobStateDto,
    pub message: Option<String>,
    pub waiting_for: Option<ModelRole>,
}

pub struct RunSummary {
    pub jobs: Vec<JobOutcome>,
    /// Jobs still queued (e.g. waiting for a retry or a model to be set up).
    pub pending: bool,
}

fn err(e: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!(e.to_string())
}

impl From<ModelRole> for Role {
    fn from(r: ModelRole) -> Self {
        match r {
            ModelRole::Router => Role::Router,
            ModelRole::Ingest => Role::Ingest,
            ModelRole::Chat => Role::Chat,
            ModelRole::Voice => Role::Voice,
            ModelRole::Vision => Role::Vision,
            ModelRole::Reflect => Role::Reflect,
            ModelRole::Lint => Role::Lint,
            ModelRole::Stt => Role::Stt,
            ModelRole::Tts => Role::Tts,
            ModelRole::Embedding => Role::Embedding,
        }
    }
}

fn role_dto(r: Role) -> ModelRole {
    match r {
        Role::Router => ModelRole::Router,
        Role::Ingest => ModelRole::Ingest,
        Role::Chat => ModelRole::Chat,
        Role::Voice => ModelRole::Voice,
        Role::Vision => ModelRole::Vision,
        Role::Reflect => ModelRole::Reflect,
        Role::Lint => ModelRole::Lint,
        Role::Stt => ModelRole::Stt,
        Role::Tts => ModelRole::Tts,
        Role::Embedding => ModelRole::Embedding,
    }
}

fn warning_dto(role: Role, model: &str) -> Option<CapabilityWarning> {
    providers::capability_warning(role, model)?;
    Some(match role {
        Role::Vision => CapabilityWarning::NoVision,
        Role::Stt => CapabilityWarning::NotSpeechToText,
        Role::Tts => CapabilityWarning::NotTextToSpeech,
        Role::Embedding => CapabilityWarning::NotEmbedding,
        _ => CapabilityWarning::NotConversational,
    })
}

fn to_config(p: AiProvider) -> ProviderConfig {
    ProviderConfig {
        id: p.id,
        name: p.name.trim().to_owned(),
        kind: match p.kind {
            ProviderKindDto::OpenaiCompatible => ProviderKind::OpenaiCompatible,
            ProviderKindDto::Anthropic => ProviderKind::Anthropic,
            ProviderKindDto::Gemini => ProviderKind::Gemini,
        },
        base_url: p.base_url.trim().to_owned(),
        extra_headers: p
            .headers
            .into_iter()
            .filter(|h| !h.name.trim().is_empty())
            .map(|h| (h.name.trim().to_owned(), h.value))
            .collect::<BTreeMap<_, _>>(),
        timeout_s: u64::from(p.timeout_s.max(5)),
    }
}

fn from_config(p: &ProviderConfig) -> Option<AiProvider> {
    Some(AiProvider {
        id: p.id.clone(),
        name: p.name.clone(),
        kind: match p.kind {
            ProviderKind::OpenaiCompatible => ProviderKindDto::OpenaiCompatible,
            ProviderKind::Anthropic => ProviderKindDto::Anthropic,
            ProviderKind::Gemini => ProviderKindDto::Gemini,
            // Test-only providers are not editable in the app.
            ProviderKind::Mock => return None,
        },
        base_url: p.base_url.clone(),
        headers: p
            .extra_headers
            .iter()
            .map(|(k, v)| HttpHeader {
                name: k.clone(),
                value: v.clone(),
            })
            .collect(),
        timeout_s: p.timeout_s as u32,
    })
}

fn keys(k: Vec<ApiKey>) -> HashMap<String, String> {
    k.into_iter()
        .filter(|k| !k.key.is_empty())
        .map(|k| (k.provider_id, k.key))
        .collect()
}

/// The public endpoint used when a provider's base URL is left empty.
#[flutter_rust_bridge::frb(sync)]
pub fn default_base_url(kind: ProviderKindDto) -> String {
    match kind {
        ProviderKindDto::OpenaiCompatible => "https://api.openai.com/v1",
        ProviderKindDto::Anthropic => "https://api.anthropic.com/v1",
        ProviderKindDto::Gemini => "https://generativelanguage.googleapis.com/v1beta",
    }
    .to_owned()
}

/// Capability warning for a model typed or picked for `role`, before it is saved.
#[flutter_rust_bridge::frb(sync)]
pub fn capability_warning(role: ModelRole, model: String) -> Option<CapabilityWarning> {
    warning_dto(role.into(), &model)
}

/// Lists a provider's models (`/models`). Works for unsaved providers, so the key can be checked
/// while the provider is being set up.
pub async fn list_models(
    provider: AiProvider,
    api_key: Option<String>,
) -> anyhow::Result<Vec<String>> {
    let p = providers::build(&to_config(provider), api_key).map_err(err)?;
    p.list_models().await.map_err(err)
}

impl LibraryHandle {
    pub fn ai_settings(&self) -> anyhow::Result<AiSettings> {
        let ai = self.session().library().config().map_err(err)?.ai;
        let roles = Role::ALL
            .iter()
            .map(|&role| {
                let rc = ai.role(role);
                RoleSetting {
                    role: role_dto(role),
                    provider_id: rc.map(|r| r.provider.clone()),
                    model: rc.map(|r| r.model.clone()),
                    inherited_from: ai.inherited_from(role).map(role_dto),
                    warning: rc.and_then(|r| warning_dto(role, &r.model)),
                }
            })
            .collect();
        Ok(AiSettings {
            providers: ai.providers.iter().filter_map(from_config).collect(),
            roles,
        })
    }

    /// Adds or updates a provider; returns its id (new providers get one).
    pub fn save_provider(&self, provider: AiProvider) -> anyhow::Result<String> {
        let cfg = to_config(provider);
        if cfg.name.is_empty() {
            return Err(anyhow::anyhow!("Give the provider a name."));
        }
        self.session()
            .library()
            .update_config(|c| c.ai.upsert_provider(cfg))
            .map_err(err)
    }

    /// Removes a provider and the roles that used it. The app deletes its key from secure storage.
    pub fn remove_provider(&self, id: String) -> anyhow::Result<()> {
        self.session()
            .library()
            .update_config(|c| c.ai.remove_provider(&id))
            .map_err(err)
    }

    pub fn set_role(
        &self,
        role: ModelRole,
        provider_id: String,
        model: String,
    ) -> anyhow::Result<()> {
        if model.trim().is_empty() {
            return Err(anyhow::anyhow!("Choose or type a model."));
        }
        self.session()
            .library()
            .update_config(|c| {
                if c.ai.provider(&provider_id).is_none() {
                    return Err(anyhow::anyhow!("That provider was removed."));
                }
                c.ai.set_role(role.into(), &provider_id, &model);
                Ok(())
            })
            .map_err(err)?
    }

    pub fn clear_role(&self, role: ModelRole) -> anyhow::Result<()> {
        self.session()
            .library()
            .update_config(|c| c.ai.clear_role(role.into()))
            .map_err(err)
    }

    /// The Test button: one real minimal call for the role as configured, with its latency.
    /// A failure is an outcome, not an error, so the app can show it next to the role.
    pub async fn test_role(
        &self,
        role: ModelRole,
        api_keys: Vec<ApiKey>,
    ) -> anyhow::Result<ProbeOutcome> {
        let rt = self.session().runtime(keys(api_keys)).map_err(err)?;
        let started = std::time::Instant::now();
        let res = match rt.for_role(role.into()) {
            Ok((p, rc)) => providers::probe(&p, &rc).await,
            Err(e) => Err(e),
        };
        Ok(match res {
            Ok(r) => ProbeOutcome {
                ok: true,
                latency_ms: r.latency_ms as u32,
                detail: r.detail,
            },
            Err(e) => ProbeOutcome {
                ok: false,
                latency_ms: started.elapsed().as_millis() as u32,
                detail: e.message,
            },
        })
    }

    /// Runs queued AI jobs (transcribe, describe, ingest) until the queue is empty or blocked.
    pub async fn run_jobs(
        &self,
        api_keys: Vec<ApiKey>,
        online: bool,
    ) -> anyhow::Result<RunSummary> {
        let session = self.session();
        let rt = session.runtime(keys(api_keys)).map_err(err)?;
        let reports = session
            .run_jobs(&rt, online, &Cancel::default())
            .await
            .map_err(err)?;
        Ok(RunSummary {
            jobs: reports
                .into_iter()
                .map(|r| JobOutcome {
                    kind: match r.kind {
                        JobKind::Transcribe => JobKindDto::Transcribe,
                        JobKind::Describe => JobKindDto::Describe,
                        JobKind::Ingest => JobKindDto::Ingest,
                        _ => JobKindDto::Other,
                    },
                    raw_id: r.raw_id,
                    state: match r.state {
                        JobState::Queued => JobStateDto::Queued,
                        JobState::Running => JobStateDto::Running,
                        JobState::Done => JobStateDto::Done,
                        JobState::Failed => JobStateDto::Failed,
                    },
                    message: r.message,
                    waiting_for: r.waiting_for.map(role_dto),
                })
                .collect(),
            pending: session.has_pending_jobs().map_err(err)?,
        })
    }

    /// Makes a failed capture's jobs runnable again (the Retry action on a failed capture).
    pub fn retry_capture(&self, raw_id: String) -> anyhow::Result<()> {
        self.session().retry_capture(&raw_id).map_err(err)
    }
}
