//! Scripted provider for tests, fixtures and `daftar eval` (§13 "mock provider that replays recorded
//! responses").
//!
//! A script is a list of chat replies, consumed in order; a reply with `when` is only used when that
//! text appears in the request (system prompt or any message), which lets router and ingest calls
//! interleave deterministically. Placeholders are filled from the conversation so recorded replies
//! stay valid against real tool results:
//!
//! * `{{hash:PATH}}` — the `hash:` printed by the latest `page_read`/`page_create` result for PATH
//! * `{{raw_path}}` / `{{raw_id}}` — from the `path:` / `id:` lines of the first user message

use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ChatRequest, ChatResponse, LlmProvider, MsgRole, OnDelta, ProviderError, ProviderErrorKind, ProviderResult, StopReason, ToolCall};
use crate::ledger::Usage;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScriptedCall {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScriptedReply {
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub tool_calls: Vec<ScriptedCall>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Script {
    #[serde(default)]
    pub chat: Vec<ScriptedReply>,
    /// Returned by `transcribe`, in order (the last one repeats).
    #[serde(default)]
    pub transcripts: Vec<String>,
}

type ChatFn = dyn Fn(&ChatRequest) -> ChatResponse + Send + Sync;

pub struct MockProvider {
    script: Mutex<Script>,
    used: Mutex<Vec<bool>>,
    transcript_cursor: Mutex<usize>,
    func: Option<Box<ChatFn>>,
    /// Every request seen, for assertions.
    pub requests: Mutex<Vec<ChatRequest>>,
}

impl MockProvider {
    pub fn new(script: Script) -> Self {
        let n = script.chat.len();
        Self { script: Mutex::new(script), used: Mutex::new(vec![false; n]), transcript_cursor: Mutex::new(0), func: None, requests: Mutex::new(vec![]) }
    }

    pub fn from_file(path: &str) -> crate::Result<Self> {
        let bytes = std::fs::read(Path::new(path)).map_err(|e| crate::Error::invalid(format!("mock script {path}: {e}")))?;
        Ok(Self::new(serde_json::from_slice(&bytes)?))
    }

    /// Fully programmable responses (unit tests).
    pub fn with_fn(f: impl Fn(&ChatRequest) -> ChatResponse + Send + Sync + 'static) -> Self {
        let mut m = Self::new(Script::default());
        m.func = Some(Box::new(f));
        m
    }

    pub fn remaining(&self) -> usize {
        self.used.lock().expect("lock").iter().filter(|u| !**u).count()
    }
}

fn request_text(req: &ChatRequest) -> String {
    let mut s = req.system.clone();
    for m in &req.messages {
        s.push('\n');
        s.push_str(&m.text());
    }
    s
}

fn fill(template: &str, req: &ChatRequest) -> String {
    let mut out = template.to_owned();
    while let Some(start) = out.find("{{hash:") {
        let Some(end) = out[start..].find("}}") else { break };
        let path = out[start + 7..start + end].to_owned();
        let needle = format!("path: {path}\n");
        let hash = req
            .messages
            .iter()
            .rev()
            .filter(|m| m.role == MsgRole::Tool)
            .map(|m| m.text())
            .find_map(|t| t.find(&needle).and_then(|i| t[i + needle.len()..].lines().next().and_then(|l| l.strip_prefix("hash: ").map(str::to_owned))))
            .unwrap_or_default();
        out.replace_range(start..start + end + 2, &hash);
    }
    let first_user = req.messages.iter().find(|m| m.role == MsgRole::User).map(|m| m.text()).unwrap_or_default();
    let field = |k: &str| first_user.lines().find_map(|l| l.strip_prefix(k)).map(|v| v.trim().to_owned()).unwrap_or_default();
    out.replace("{{raw_path}}", &field("path:")).replace("{{raw_id}}", &field("id:"))
}

fn fill_value(v: &Value, req: &ChatRequest) -> Value {
    match v {
        Value::String(s) => Value::String(fill(s, req)),
        Value::Array(a) => Value::Array(a.iter().map(|x| fill_value(x, req)).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, x)| (k.clone(), fill_value(x, req))).collect()),
        other => other.clone(),
    }
}

#[async_trait::async_trait]
impl LlmProvider for MockProvider {
    fn name(&self) -> &str {
        "Mock"
    }

    async fn chat(&self, req: &ChatRequest, on_delta: OnDelta<'_>) -> ProviderResult<ChatResponse> {
        self.requests.lock().expect("lock").push(req.clone());
        let resp = if let Some(f) = &self.func {
            f(req)
        } else {
            let text = request_text(req);
            let script = self.script.lock().expect("lock");
            let mut used = self.used.lock().expect("lock");
            let idx = script
                .chat
                .iter()
                .enumerate()
                .find(|(i, r)| !used[*i] && r.when.as_deref().is_none_or(|w| text.contains(w)))
                .map(|(i, _)| i)
                .ok_or_else(|| ProviderError::new(ProviderErrorKind::BadRequest, "Mock script has no reply left for this request."))?;
            used[idx] = true;
            let r = &script.chat[idx];
            let tool_calls: Vec<ToolCall> = r
                .tool_calls
                .iter()
                .enumerate()
                .map(|(i, c)| ToolCall { id: format!("mock_{idx}_{i}"), name: c.name.clone(), arguments: fill_value(&c.arguments, req) })
                .collect();
            ChatResponse {
                text: fill(&r.text, req),
                stop: if tool_calls.is_empty() { StopReason::EndTurn } else { StopReason::ToolUse },
                tool_calls,
                usage: Usage { input_tokens: (text.len() / 4) as u64, output_tokens: 50, cached_input_tokens: 0, cost_usd: None },
            }
        };
        if let Some(cb) = on_delta {
            for w in resp.text.split_inclusive(' ') {
                cb(w);
            }
        }
        Ok(resp)
    }

    async fn transcribe(&self, _model: &str, _audio: Vec<u8>, _file_name: &str, _language: Option<&str>) -> ProviderResult<String> {
        let script = self.script.lock().expect("lock");
        let mut c = self.transcript_cursor.lock().expect("lock");
        let t = script.transcripts.get(*c).or(script.transcripts.last()).cloned().ok_or_else(|| ProviderError::new(ProviderErrorKind::NotSupported, "Mock has no transcripts."))?;
        *c += 1;
        Ok(t)
    }

    async fn list_models(&self) -> ProviderResult<Vec<String>> {
        Ok(vec!["mock-small".into(), "mock-large".into()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::Message;

    fn req(msgs: Vec<Message>) -> ChatRequest {
        ChatRequest { model: "m".into(), system: "INGEST".into(), messages: msgs, tools: vec![], max_tokens: 100, temperature: None, json: false, params: Default::default() }
    }

    #[tokio::test]
    async fn fills_placeholders_and_honours_when() {
        let script: Script = serde_json::from_value(serde_json::json!({
            "chat": [
                {"when": "ROUTER", "text": "routed"},
                {"tool_calls": [{"name": "page_edit", "arguments": {"path": "a.md", "base_hash": "{{hash:a.md}}", "src": "{{raw_path}}"}}]}
            ]
        }))
        .unwrap();
        let m = MockProvider::new(script);
        let call = ToolCall { id: "1".into(), name: "page_read".into(), arguments: Value::Null };
        let r = m
            .chat(&req(vec![Message::user("id: 01X\npath: raw/2026/x.md\n"), Message::assistant("", vec![call.clone()]), Message::tool(&call, "path: a.md\nhash: abc123\n 1 | x")]), None)
            .await
            .unwrap();
        assert_eq!(r.tool_calls[0].arguments["base_hash"], "abc123");
        assert_eq!(r.tool_calls[0].arguments["src"], "raw/2026/x.md");
        assert_eq!(m.remaining(), 1, "ROUTER reply skipped because the request didn't mention it");
    }
}
