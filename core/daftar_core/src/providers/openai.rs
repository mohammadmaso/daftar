//! OpenAI-compatible adapter: OpenAI, OpenRouter, Groq, Together, DeepSeek, Azure-style gateways,
//! LiteLLM, … (Chat Completions + audio + embeddings + models).

use std::collections::BTreeMap;
use std::time::Duration;

use serde_json::{Value, json};

use super::sse;
use super::{
    ChatRequest, ChatResponse, LlmProvider, MsgRole, OnDelta, Part, ProviderConfig, ProviderError, ProviderErrorKind,
    ProviderResult, StopReason, ToolCall,
};
use crate::ledger::Usage;

pub struct OpenAiCompatible {
    name: String,
    base: String,
    key: String,
    headers: BTreeMap<String, String>,
    http: reqwest::Client,
}

impl OpenAiCompatible {
    pub fn new(c: &ProviderConfig, key: String, timeout: Duration) -> Self {
        let base = if c.base_url.is_empty() { "https://api.openai.com/v1".to_owned() } else { c.base_url.trim_end_matches('/').to_owned() };
        Self { name: c.name.clone(), base, key, headers: c.extra_headers.clone(), http: crate::tls::http_client(timeout) }
    }

    fn req(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let mut r = self.http.request(method, format!("{}{path}", self.base));
        if !self.key.is_empty() {
            r = r.bearer_auth(&self.key);
        }
        for (k, v) in &self.headers {
            r = r.header(k, v);
        }
        r
    }

    fn body(&self, req: &ChatRequest, stream_options: bool) -> Value {
        let mut messages = vec![json!({"role": "system", "content": req.system})];
        for m in &req.messages {
            match m.role {
                MsgRole::User => {
                    let content: Vec<Value> = m
                        .parts
                        .iter()
                        .map(|p| match p {
                            Part::Text { text } => json!({"type": "text", "text": text}),
                            Part::Image { media_type, data } => {
                                json!({"type": "image_url", "image_url": {"url": format!("data:{media_type};base64,{data}")}})
                            }
                        })
                        .collect();
                    messages.push(json!({"role": "user", "content": content}));
                }
                MsgRole::Assistant => {
                    let mut msg = json!({"role": "assistant", "content": m.text()});
                    if !m.tool_calls.is_empty() {
                        msg["tool_calls"] = m
                            .tool_calls
                            .iter()
                            .map(|c| json!({"id": c.id, "type": "function", "function": {"name": c.name, "arguments": c.arguments.to_string()}}))
                            .collect();
                    }
                    messages.push(msg);
                }
                MsgRole::Tool => messages.push(json!({"role": "tool", "tool_call_id": m.tool_call_id, "content": m.text()})),
            }
        }
        let mut body = json!({
            "model": req.model,
            "messages": messages,
            "stream": true,
        });
        // OpenAI's own API wants max_completion_tokens; most compatible gateways still take max_tokens.
        let field = if self.base.contains("api.openai.com") { "max_completion_tokens" } else { "max_tokens" };
        body[field] = json!(req.max_tokens);
        if stream_options {
            body["stream_options"] = json!({"include_usage": true});
        }
        if let Some(t) = req.temperature {
            body["temperature"] = json!(t);
        }
        if !req.tools.is_empty() {
            body["tools"] = req
                .tools
                .iter()
                .map(|t| json!({"type": "function", "function": {"name": t.name, "description": t.description, "parameters": t.parameters}}))
                .collect();
        }
        if req.json {
            body["response_format"] = json!({"type": "json_object"});
        }
        for (k, v) in &req.params {
            body[k] = v.clone();
        }
        body
    }

    async fn send_chat(&self, req: &ChatRequest, on_delta: OnDelta<'_>, stream_options: bool) -> ProviderResult<ChatResponse> {
        let resp = self
            .req(reqwest::Method::POST, "/chat/completions")
            .json(&self.body(req, stream_options))
            .send()
            .await
            .map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        let resp = check(&self.name, resp).await?;
        let mut text = String::new();
        let mut calls: BTreeMap<u64, (String, String, String)> = BTreeMap::new();
        let mut usage = Usage::default();
        let mut stop = StopReason::Other;
        sse::read(&self.name, resp, |ev| {
            if ev.data == "[DONE]" {
                return Ok(());
            }
            let v: Value = serde_json::from_str(&ev.data).map_err(|_| ProviderError::new(ProviderErrorKind::Server, format!("{} sent malformed stream data.", self.name)))?;
            if let Some(err) = v.get("error") {
                return Err(ProviderError::new(ProviderErrorKind::Server, format!("{}: {}", self.name, err.get("message").and_then(Value::as_str).unwrap_or("stream error"))));
            }
            if let Some(u) = v.get("usage").filter(|u| !u.is_null()) {
                usage.input_tokens = u["prompt_tokens"].as_u64().unwrap_or(0);
                usage.output_tokens = u["completion_tokens"].as_u64().unwrap_or(0);
                usage.cached_input_tokens = u.pointer("/prompt_tokens_details/cached_tokens").and_then(Value::as_u64).unwrap_or(0);
            }
            let Some(choice) = v.pointer("/choices/0") else { return Ok(()) };
            if let Some(d) = choice.pointer("/delta/content").and_then(Value::as_str) {
                if !d.is_empty() {
                    text.push_str(d);
                    if let Some(cb) = on_delta {
                        cb(d);
                    }
                }
            }
            if let Some(tcs) = choice.pointer("/delta/tool_calls").and_then(Value::as_array) {
                for tc in tcs {
                    let idx = tc["index"].as_u64().unwrap_or(0);
                    let e = calls.entry(idx).or_default();
                    if let Some(id) = tc["id"].as_str() {
                        e.0 = id.to_owned();
                    }
                    if let Some(n) = tc.pointer("/function/name").and_then(Value::as_str) {
                        e.1.push_str(n);
                    }
                    if let Some(a) = tc.pointer("/function/arguments").and_then(Value::as_str) {
                        e.2.push_str(a);
                    }
                }
            }
            if let Some(fr) = choice["finish_reason"].as_str() {
                stop = match fr {
                    "stop" => StopReason::EndTurn,
                    "tool_calls" | "function_call" => StopReason::ToolUse,
                    "length" => StopReason::MaxTokens,
                    _ => StopReason::Other,
                };
            }
            Ok(())
        })
        .await?;
        let tool_calls = calls
            .into_iter()
            .map(|(i, (id, name, args))| ToolCall {
                id: if id.is_empty() { format!("call_{i}") } else { id },
                name,
                arguments: parse_args(&args),
            })
            .collect::<Vec<_>>();
        if !tool_calls.is_empty() {
            stop = StopReason::ToolUse;
        }
        Ok(ChatResponse { text, tool_calls, usage, stop })
    }
}

/// Tool arguments arrive as a JSON string; tolerate empty strings from some gateways.
pub(crate) fn parse_args(s: &str) -> Value {
    if s.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(s).unwrap_or_else(|_| json!({"_unparsed": s}))
    }
}

pub(crate) async fn check(provider: &str, resp: reqwest::Response) -> ProviderResult<reqwest::Response> {
    if resp.status().is_success() {
        return Ok(resp);
    }
    let status = resp.status().as_u16();
    let retry = resp.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|s| s.parse().ok());
    let body = resp.text().await.unwrap_or_default();
    Err(ProviderError::from_status(provider, status, &body, retry))
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiCompatible {
    fn name(&self) -> &str {
        &self.name
    }

    async fn chat(&self, req: &ChatRequest, on_delta: OnDelta<'_>) -> ProviderResult<ChatResponse> {
        match self.send_chat(req, on_delta, true).await {
            // Some gateways reject `stream_options`; retry once without it.
            Err(e) if e.kind == ProviderErrorKind::BadRequest && e.message.contains("stream_options") => self.send_chat(req, on_delta, false).await,
            other => other,
        }
    }

    async fn transcribe(&self, model: &str, audio: Vec<u8>, file_name: &str, language: Option<&str>) -> ProviderResult<String> {
        let mut form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(audio).file_name(file_name.to_owned()))
            .text("model", model.to_owned())
            .text("response_format", "json");
        if let Some(l) = language.filter(|l| *l != "auto") {
            form = form.text("language", l.to_owned());
        }
        let resp = self.req(reqwest::Method::POST, "/audio/transcriptions").multipart(form).send().await.map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        let v: Value = check(&self.name, resp).await?.json().await.map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        Ok(v["text"].as_str().unwrap_or_default().trim().to_owned())
    }

    async fn speech(&self, model: &str, voice: &str, text: &str) -> ProviderResult<Vec<u8>> {
        let resp = self
            .req(reqwest::Method::POST, "/audio/speech")
            .json(&json!({"model": model, "voice": voice, "input": text, "response_format": "mp3"}))
            .send()
            .await
            .map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        let bytes = check(&self.name, resp).await?.bytes().await.map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        Ok(bytes.to_vec())
    }

    async fn embed(&self, model: &str, inputs: &[String]) -> ProviderResult<Vec<Vec<f32>>> {
        let resp = self
            .req(reqwest::Method::POST, "/embeddings")
            .json(&json!({"model": model, "input": inputs}))
            .send()
            .await
            .map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        let v: Value = check(&self.name, resp).await?.json().await.map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        Ok(v["data"]
            .as_array()
            .map(|a| a.iter().map(|d| d["embedding"].as_array().map(|e| e.iter().filter_map(Value::as_f64).map(|x| x as f32).collect()).unwrap_or_default()).collect())
            .unwrap_or_default())
    }

    async fn list_models(&self) -> ProviderResult<Vec<String>> {
        let resp = self.req(reqwest::Method::GET, "/models").send().await.map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        let v: Value = check(&self.name, resp).await?.json().await.map_err(|e| ProviderError::from_reqwest(&self.name, e))?;
        let mut ids: Vec<String> = v["data"].as_array().map(|a| a.iter().filter_map(|m| m["id"].as_str().map(str::to_owned)).collect()).unwrap_or_default();
        ids.sort();
        Ok(ids)
    }
}
