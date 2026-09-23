//! The tool-calling agent loop (§6.1), shared by ingest, query, lint, reflect and voice.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::ledger::Usage;
use crate::providers::{ChatRequest, DynProvider, Message, MsgRole, OnDelta, Part, ProviderError, ToolSpec};
use crate::tools::OpContext;

#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

pub struct AgentSpec {
    pub model: String,
    pub system: String,
    pub tools: Vec<ToolSpec>,
    pub max_steps: usize,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
    /// Rough context budget in characters (≈ 4 chars per token).
    pub context_chars: usize,
    pub params: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error("The assistant used too many steps without finishing.")]
    StepBudget,
    #[error("Cancelled.")]
    Cancelled,
    #[error("The assistant's changes failed validation: {0}")]
    Invalid(String),
}

pub struct AgentOutcome {
    pub text: String,
    pub usage: Usage,
    pub steps: usize,
    pub messages: Vec<Message>,
}

/// Validation hook: returns error messages (empty = accept).
pub type Validator<'v> = &'v dyn Fn(&OpContext<'_>) -> Vec<String>;

pub async fn run(
    provider: &DynProvider,
    spec: &AgentSpec,
    ctx: &mut OpContext<'_>,
    mut messages: Vec<Message>,
    validator: Option<Validator<'_>>,
    max_repairs: usize,
    cancel: &Cancel,
    on_delta: OnDelta<'_>,
) -> Result<AgentOutcome, AgentError> {
    let mut usage = Usage::default();
    let mut repairs = 0;
    for step in 1..=spec.max_steps {
        if cancel.is_cancelled() {
            return Err(AgentError::Cancelled);
        }
        trim_context(&mut messages, spec.context_chars);
        let req = ChatRequest {
            model: spec.model.clone(),
            system: spec.system.clone(),
            messages: messages.clone(),
            tools: spec.tools.clone(),
            max_tokens: spec.max_tokens,
            temperature: spec.temperature,
            json: false,
            params: spec.params.clone(),
        };
        let resp = provider.chat(&req, on_delta).await?;
        usage.input_tokens += resp.usage.input_tokens;
        usage.output_tokens += resp.usage.output_tokens;
        usage.cached_input_tokens += resp.usage.cached_input_tokens;
        messages.push(Message::assistant(resp.text.clone(), resp.tool_calls.clone()));

        if resp.tool_calls.is_empty() {
            if let Some(v) = validator {
                let errors = v(ctx);
                if !errors.is_empty() {
                    if repairs >= max_repairs {
                        return Err(AgentError::Invalid(errors.join(" · ")));
                    }
                    repairs += 1;
                    messages.push(Message::user(format!(
                        "The changes were rejected by the validator. Fix every problem below with the tools, then finish again.\n- {}",
                        errors.join("\n- ")
                    )));
                    continue;
                }
            }
            return Ok(AgentOutcome { text: resp.text, usage, steps: step, messages });
        }
        let mut images = Vec::new();
        for call in &resp.tool_calls {
            if cancel.is_cancelled() {
                return Err(AgentError::Cancelled);
            }
            let out = ctx.call(&call.name, &call.arguments);
            messages.push(Message::tool(call, out.text));
            if let Some((media_type, data)) = out.image {
                images.push(Part::Image { media_type, data });
            }
        }
        // Tool results are text-only on most APIs; images follow as a user turn.
        if !images.is_empty() {
            let mut m = Message::user("Here is the image you asked to view.");
            m.parts.extend(images);
            messages.push(m);
        }
    }
    Err(AgentError::StepBudget)
}

fn size(m: &Message) -> usize {
    m.parts
        .iter()
        .map(|p| match p {
            Part::Text { text } => text.len(),
            Part::Image { .. } => 4_000,
        })
        .sum::<usize>()
        + m.tool_calls.iter().map(|c| c.arguments.to_string().len()).sum::<usize>()
}

fn path_of_tool_result(text: &str) -> Option<&str> {
    text.lines().take(3).find_map(|l| l.strip_prefix("path: "))
}

/// Keeps the conversation within budget (§6.1): earlier reads of a page that was edited later are
/// replaced by a note, then the oldest tool results are dropped first.
pub fn trim_context(messages: &mut [Message], budget: usize) {
    // 1. Stale reads: a page_read result for PATH followed later by an edit/create/claim on PATH.
    let n = messages.len();
    for i in 0..n {
        if messages[i].role != MsgRole::Tool || messages[i].tool_name.as_deref() != Some("page_read") {
            continue;
        }
        let text = messages[i].text();
        let Some(path) = path_of_tool_result(&text).map(str::to_owned) else { continue };
        let edited_later = messages[i + 1..].iter().any(|m| {
            m.role == MsgRole::Tool
                && matches!(m.tool_name.as_deref(), Some("page_edit" | "page_create" | "claim_propose" | "claim_supersede"))
                && path_of_tool_result(&m.text()) == Some(path.as_str())
                && !m.text().starts_with("ERROR")
        });
        if edited_later {
            messages[i].parts = vec![Part::Text { text: format!("[earlier copy of {path} removed because the page was edited since; page_read it again if needed]") }];
        }
    }
    // 2. Budget: drop oldest tool results (never the first user message or the last few turns).
    let mut total: usize = messages.iter().map(size).sum();
    let keep_tail = 6;
    let mut i = 1;
    while total > budget && i + keep_tail < messages.len() {
        if messages[i].role == MsgRole::Tool && size(&messages[i]) > 200 {
            let before = size(&messages[i]);
            messages[i].parts = vec![Part::Text { text: "[older tool result removed to save space]".into() }];
            total = total - before + size(&messages[i]);
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::ToolCall;

    #[test]
    fn stale_reads_are_replaced() {
        let read = ToolCall { id: "1".into(), name: "page_read".into(), arguments: serde_json::Value::Null };
        let edit = ToolCall { id: "2".into(), name: "page_edit".into(), arguments: serde_json::Value::Null };
        let mut ms = vec![
            Message::user("go"),
            Message::assistant("", vec![read.clone()]),
            Message::tool(&read, "path: vaults/life/people/sara.md\nhash: aaa\n 1 | x"),
            Message::assistant("", vec![edit.clone()]),
            Message::tool(&edit, "edited\npath: vaults/life/people/sara.md\nhash: bbb"),
        ];
        trim_context(&mut ms, 1_000_000);
        assert!(ms[2].text().starts_with("[earlier copy of vaults/life/people/sara.md removed"));
        assert!(ms[4].text().starts_with("edited"));
    }
}
