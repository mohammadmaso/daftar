//! Google Gemini generateContent (streaming via `alt=sse`) with tools and vision.

use std::collections::BTreeMap;
use std::time::Duration;

use serde_json::{Value, json};

use super::openai::check;
use super::sse;
use super::{
    ChatRequest, ChatResponse, LlmProvider, MsgRole, OnDelta, Part, ProviderConfig, ProviderError,
    ProviderResult, StopReason, ToolCall,
};
use crate::ledger::Usage;

pub struct Gemini {
    name: String,
    base: String,
    key: String,
    headers: BTreeMap<String, String>,
    http: reqwest::Client,
}

impl Gemini {
    pub fn new(c: &ProviderConfig, key: String, timeout: Duration) -> Self {
        let base = if c.base_url.is_empty() {
            "https://generativelanguage.googleapis.com/v1beta".to_owned()
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
            .header("x-goog-api-key", &self.key);
        for (k, v) in &self.headers {
            r = r.header(k, v);
        }
        r
    }

    pub(crate) fn body(req: &ChatRequest) -> Value {
        let mut contents: Vec<Value> = Vec::new();
        for m in &req.messages {
            let (role, parts): (&str, Vec<Value>) = match m.role {
                MsgRole::User => (
                    "user",
                    m.parts
                        .iter()
                        .map(|p| match p {
                            Part::Text { text } => json!({"text": text}),
                            Part::Image { media_type, data } => {
                                json!({"inline_data": {"mime_type": media_type, "data": data}})
                            }
                        })
                        .collect(),
                ),
                MsgRole::Assistant => {
                    let mut ps = Vec::new();
                    let t = m.text();
                    if !t.is_empty() {
                        ps.push(json!({"text": t}));
                    }
                    for tc in &m.tool_calls {
                        ps.push(json!({"functionCall": {"name": tc.name, "args": tc.arguments}}));
                    }
                    ("model", ps)
                }
                MsgRole::Tool => (
                    "user",
                    vec![
                        json!({"functionResponse": {"name": m.tool_name, "response": {"content": m.text()}}}),
                    ],
                ),
            };
            if let Some(last) = contents.last_mut() {
                if last["role"] == role && m.role == MsgRole::Tool {
                    last["parts"].as_array_mut().expect("array").extend(parts);
                    continue;
                }
            }
            contents.push(json!({"role": role, "parts": parts}));
        }
        let mut generation = json!({"maxOutputTokens": req.max_tokens});
        if let Some(t) = req.temperature {
            generation["temperature"] = json!(t);
        }
        if req.json {
            generation["responseMimeType"] = json!("application/json");
        }
        let mut body = json!({
            "systemInstruction": {"parts": [{"text": req.system}]},
            "contents": contents,
            "generationConfig": generation,
        });
        if !req.tools.is_empty() {
            body["tools"] = json!([{"functionDeclarations": req.tools.iter().map(|t| json!({"name": t.name, "description": t.description, "parameters": strip_unsupported(&t.parameters)})).collect::<Vec<_>>()}]);
        }
        body
    }
}

/// Gemini's schema dialect rejects some JSON-Schema keywords.
fn strip_unsupported(v: &Value) -> Value {
    match v {
        Value::Object(o) => Value::Object(
            o.iter()
                .filter(|(k, _)| {
                    !matches!(k.as_str(), "additionalProperties" | "$schema" | "default")
                })
                .map(|(k, v)| (k.clone(), strip_unsupported(v)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(strip_unsupported).collect()),
        other => other.clone(),
    }
}

#[async_trait::async_trait]
impl LlmProvider for Gemini {
    fn name(&self) -> &str {
        &self.name
    }

    async fn chat(&self, req: &ChatRequest, on_delta: OnDelta<'_>) -> ProviderResult<ChatResponse> {
        let path = format!("/models/{}:streamGenerateContent?alt=sse", req.model);
        let resp = self
            .req(reqwest::Method::POST, &path)
            .json(&Self::body(req))
            .send()
            .await
            .map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        let resp = check(&self.name, resp).await?;
        let mut text = String::new();
        let mut calls = Vec::new();
        let mut usage = Usage::default();
        let mut stop = StopReason::Other;
        sse::read(&self.name, resp, |ev| {
            let Ok(v) = serde_json::from_str::<Value>(&ev.data) else {
                return Ok(());
            };
            if let Some(u) = v.get("usageMetadata") {
                usage.input_tokens = u["promptTokenCount"].as_u64().unwrap_or(usage.input_tokens);
                usage.output_tokens = u["candidatesTokenCount"]
                    .as_u64()
                    .unwrap_or(usage.output_tokens);
                usage.cached_input_tokens = u["cachedContentTokenCount"].as_u64().unwrap_or(0);
            }
            if let Some(parts) = v
                .pointer("/candidates/0/content/parts")
                .and_then(Value::as_array)
            {
                for p in parts {
                    if let Some(t) = p["text"].as_str() {
                        text.push_str(t);
                        if let Some(cb) = on_delta {
                            cb(t);
                        }
                    }
                    if let Some(fc) = p.get("functionCall") {
                        calls.push(ToolCall {
                            id: format!("call_{}", calls.len()),
                            name: fc["name"].as_str().unwrap_or_default().to_owned(),
                            arguments: fc["args"].clone(),
                        });
                    }
                }
            }
            if let Some(fr) = v
                .pointer("/candidates/0/finishReason")
                .and_then(Value::as_str)
            {
                stop = match fr {
                    "STOP" => StopReason::EndTurn,
                    "MAX_TOKENS" => StopReason::MaxTokens,
                    _ => StopReason::Other,
                };
            }
            Ok(())
        })
        .await?;
        if !calls.is_empty() {
            stop = StopReason::ToolUse;
        }
        Ok(ChatResponse {
            text,
            tool_calls: calls,
            usage,
            stop,
        })
    }

    async fn embed(&self, model: &str, inputs: &[String]) -> ProviderResult<Vec<Vec<f32>>> {
        let reqs: Vec<Value> = inputs.iter().map(|t| json!({"model": format!("models/{model}"), "content": {"parts": [{"text": t}]}})).collect();
        let resp = self
            .req(
                reqwest::Method::POST,
                &format!("/models/{model}:batchEmbedContents"),
            )
            .json(&json!({"requests": reqs}))
            .send()
            .await
            .map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        let v: Value = check(&self.name, resp)
            .await?
            .json()
            .await
            .map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        Ok(v["embeddings"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|e| {
                        e["values"]
                            .as_array()
                            .map(|x| {
                                x.iter()
                                    .filter_map(Value::as_f64)
                                    .map(|f| f as f32)
                                    .collect()
                            })
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn list_models(&self) -> ProviderResult<Vec<String>> {
        let resp = self
            .req(reqwest::Method::GET, "/models?pageSize=200")
            .send()
            .await
            .map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        let v: Value = check(&self.name, resp)
            .await?
            .json()
            .await
            .map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        Ok(v["models"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|m| {
                        m["name"]
                            .as_str()
                            .map(|n| n.trim_start_matches("models/").to_owned())
                    })
                    .collect()
            })
            .unwrap_or_default())
    }
}
