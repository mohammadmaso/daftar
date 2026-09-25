//! Anthropic Messages API: tools, streaming, vision, prompt caching of the system prompt.

use std::collections::BTreeMap;
use std::time::Duration;

use serde_json::{Value, json};

use super::openai::{check, parse_args};
use super::sse;
use super::{
    ChatRequest, ChatResponse, LlmProvider, MsgRole, OnDelta, Part, ProviderConfig, ProviderError,
    ProviderErrorKind, ProviderResult, StopReason, ToolCall,
};
use crate::ledger::Usage;

const VERSION: &str = "2023-06-01";

pub struct Anthropic {
    name: String,
    base: String,
    key: String,
    headers: BTreeMap<String, String>,
    http: reqwest::Client,
}

impl Anthropic {
    pub fn new(c: &ProviderConfig, key: String, timeout: Duration) -> Self {
        let base = if c.base_url.is_empty() {
            "https://api.anthropic.com/v1".to_owned()
        } else {
            c.base_url.trim_end_matches('/').to_owned()
        };
        Self {
            name: c.name.clone(),
            base,
            key,
            headers: c.extra_headers.clone(),
            http: crate::tls::http_client(timeout),
        }
    }

    fn req(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let mut r = self
            .http
            .request(method, format!("{}{path}", self.base))
            .header("x-api-key", &self.key)
            .header("anthropic-version", VERSION);
        for (k, v) in &self.headers {
            r = r.header(k, v);
        }
        r
    }

    pub(crate) fn body(req: &ChatRequest) -> Value {
        let mut messages: Vec<Value> = Vec::new();
        for m in &req.messages {
            let (role, content): (&str, Vec<Value>) = match m.role {
                MsgRole::User => (
                    "user",
                    m.parts
                        .iter()
                        .map(|p| match p {
                            Part::Text { text } => json!({"type": "text", "text": text}),
                            Part::Image { media_type, data } => json!({"type": "image", "source": {"type": "base64", "media_type": media_type, "data": data}}),
                        })
                        .collect(),
                ),
                MsgRole::Assistant => {
                    let mut c: Vec<Value> = Vec::new();
                    let t = m.text();
                    if !t.is_empty() {
                        c.push(json!({"type": "text", "text": t}));
                    }
                    for tc in &m.tool_calls {
                        c.push(json!({"type": "tool_use", "id": tc.id, "name": tc.name, "input": tc.arguments}));
                    }
                    ("assistant", c)
                }
                MsgRole::Tool => ("user", vec![json!({"type": "tool_result", "tool_use_id": m.tool_call_id, "content": m.text()})]),
            };
            // Consecutive tool results must share one user turn.
            if let Some(last) = messages.last_mut()
                && last["role"] == role
                && role == "user"
                && m.role == MsgRole::Tool
            {
                last["content"]
                    .as_array_mut()
                    .expect("array")
                    .extend(content);
                continue;
            }
            messages.push(json!({"role": role, "content": content}));
        }
        let mut body = json!({
            "model": req.model,
            "max_tokens": req.max_tokens,
            "stream": true,
            // The system prompt (schema + rules) is large and stable: cache it.
            "system": [{"type": "text", "text": req.system, "cache_control": {"type": "ephemeral"}}],
            "messages": messages,
        });
        if let Some(t) = req.temperature {
            body["temperature"] = json!(t);
        }
        if !req.tools.is_empty() {
            let mut tools: Vec<Value> = req.tools.iter().map(|t| json!({"name": t.name, "description": t.description, "input_schema": t.parameters})).collect();
            if let Some(last) = tools.last_mut() {
                last["cache_control"] = json!({"type": "ephemeral"});
            }
            body["tools"] = Value::Array(tools);
        }
        for (k, v) in &req.params {
            body[k] = v.clone();
        }
        body
    }
}

#[async_trait::async_trait]
impl LlmProvider for Anthropic {
    fn name(&self) -> &str {
        &self.name
    }

    async fn chat(&self, req: &ChatRequest, on_delta: OnDelta<'_>) -> ProviderResult<ChatResponse> {
        let mut req = req.clone();
        if req.json {
            // No JSON mode: instruct and prefill is unreliable with tools, so instruct only.
            req.system
                .push_str("\n\nReply with a single JSON object and nothing else.");
        }
        let resp = self
            .req(reqwest::Method::POST, "/messages")
            .json(&Self::body(&req))
            .send()
            .await
            .map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        let resp = check(&self.name, resp).await?;
        let mut text = String::new();
        // index -> (id, name, partial json)
        let mut blocks: BTreeMap<u64, (String, String, String)> = BTreeMap::new();
        let mut usage = Usage::default();
        let mut stop = StopReason::Other;
        sse::read(&self.name, resp, |ev| {
            let v: Value = match serde_json::from_str(&ev.data) {
                Ok(v) => v,
                Err(_) => return Ok(()),
            };
            match v["type"].as_str().unwrap_or_default() {
                "message_start" => {
                    let u = &v["message"]["usage"];
                    usage.input_tokens = u["input_tokens"].as_u64().unwrap_or(0)
                        + u["cache_read_input_tokens"].as_u64().unwrap_or(0)
                        + u["cache_creation_input_tokens"].as_u64().unwrap_or(0);
                    usage.cached_input_tokens = u["cache_read_input_tokens"].as_u64().unwrap_or(0);
                }
                "content_block_start" => {
                    let b = &v["content_block"];
                    if b["type"] == "tool_use" {
                        blocks.insert(
                            v["index"].as_u64().unwrap_or(0),
                            (
                                b["id"].as_str().unwrap_or_default().to_owned(),
                                b["name"].as_str().unwrap_or_default().to_owned(),
                                String::new(),
                            ),
                        );
                    }
                }
                "content_block_delta" => {
                    let d = &v["delta"];
                    match d["type"].as_str() {
                        Some("text_delta") => {
                            let t = d["text"].as_str().unwrap_or_default();
                            text.push_str(t);
                            if let Some(cb) = on_delta {
                                cb(t);
                            }
                        }
                        Some("input_json_delta") => {
                            if let Some(b) = blocks.get_mut(&v["index"].as_u64().unwrap_or(0)) {
                                b.2.push_str(d["partial_json"].as_str().unwrap_or_default());
                            }
                        }
                        _ => {}
                    }
                }
                "message_delta" => {
                    usage.output_tokens = v["usage"]["output_tokens"]
                        .as_u64()
                        .unwrap_or(usage.output_tokens);
                    stop = match v["delta"]["stop_reason"].as_str() {
                        Some("end_turn") | Some("stop_sequence") => StopReason::EndTurn,
                        Some("tool_use") => StopReason::ToolUse,
                        Some("max_tokens") => StopReason::MaxTokens,
                        _ => stop,
                    };
                }
                "error" => {
                    let kind = if v["error"]["type"] == "overloaded_error" {
                        ProviderErrorKind::Server
                    } else {
                        ProviderErrorKind::BadRequest
                    };
                    return Err(ProviderError::new(
                        kind,
                        format!(
                            "{}: {}",
                            self.name,
                            v["error"]["message"].as_str().unwrap_or("error")
                        ),
                    ));
                }
                _ => {}
            }
            Ok(())
        })
        .await?;
        let tool_calls: Vec<ToolCall> = blocks
            .into_values()
            .map(|(id, name, args)| ToolCall {
                id,
                name,
                arguments: parse_args(&args),
            })
            .collect();
        Ok(ChatResponse {
            text,
            tool_calls,
            usage,
            stop,
        })
    }

    async fn list_models(&self) -> ProviderResult<Vec<String>> {
        let resp = self
            .req(reqwest::Method::GET, "/models?limit=100")
            .send()
            .await
            .map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        let v: Value = check(&self.name, resp)
            .await?
            .json()
            .await
            .map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        Ok(v["data"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|m| m["id"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default())
    }
}
