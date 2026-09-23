//! A deterministic stand-in for a model, driving the real tools like an LLM would: route, read,
//! create or append, recover from stale hashes, finish. Used by the M2 scenario tests.

use std::sync::Arc;

use daftar_core::ledger::Usage;
use daftar_core::providers::{AiConfig, ChatRequest, ChatResponse, MockProvider, MsgRole, ProviderConfig, ProviderKind, Role, RoleConfig, StopReason, ToolCall};
use daftar_core::runtime::AiRuntime;
use serde_json::{Value, json};

pub fn ai_config() -> AiConfig {
    let role = |r: Role| RoleConfig { role: r, provider: "mock".into(), model: "mock-large".into(), params: Default::default() };
    AiConfig {
        providers: vec![ProviderConfig { id: "mock".into(), name: "Mock".into(), kind: ProviderKind::Mock, base_url: String::new(), extra_headers: Default::default(), timeout_s: 10 }],
        roles: vec![role(Role::Chat), role(Role::Stt), role(Role::Vision)],
        prices: Default::default(),
    }
}

pub fn runtime(p: MockProvider) -> AiRuntime {
    AiRuntime::new(ai_config(), Default::default()).with_provider("mock", Arc::new(p))
}

fn reply(text: &str, calls: Vec<(&str, Value)>) -> ChatResponse {
    let tool_calls: Vec<ToolCall> = calls.into_iter().enumerate().map(|(i, (n, a))| ToolCall { id: format!("t{i}"), name: n.into(), arguments: a }).collect();
    ChatResponse { text: text.into(), stop: if tool_calls.is_empty() { StopReason::EndTurn } else { StopReason::ToolUse }, tool_calls, usage: Usage { input_tokens: 100, output_tokens: 20, ..Default::default() } }
}

fn field<'a>(msg: &'a str, key: &str) -> &'a str {
    msg.lines().find_map(|l| l.strip_prefix(key)).map(str::trim).unwrap_or("")
}

fn capture_text(msg: &str) -> String {
    let s = msg.split("<capture>\n").nth(1).unwrap_or("");
    s.split("\n</capture>").next().unwrap_or("").to_owned()
}

/// Last tool result text for a tool call about `path` (by name order in this turn).
fn results(req: &ChatRequest) -> Vec<String> {
    req.messages.iter().filter(|m| m.role == MsgRole::Tool).map(|m| m.text()).collect()
}

fn hash_of(text: &str) -> Option<String> {
    text.lines().find_map(|l| l.strip_prefix("hash: ")).map(str::to_owned)
}

/// Files every capture into today's journal and, if it mentions Sara, into her person page.
pub fn archivist() -> MockProvider {
    MockProvider::with_fn(|req: &ChatRequest| {
        if req.system.contains("You are the ROUTER") {
            let user = req.messages[0].text();
            let fiction = user.contains("story") || user.contains("داستان");
            let targets = if fiction {
                json!([{"vault": "stories", "reason": "fiction", "confidence": 0.95}])
            } else if user.contains("headache") || user.contains("سردرد") {
                json!([{"vault": "life", "reason": "journal", "confidence": 0.9}, {"vault": "health", "reason": "symptom", "confidence": 0.8}])
            } else {
                json!([{"vault": "life", "reason": "journal", "confidence": 0.9}])
            };
            let v = json!({"targets": targets, "is_fiction": fiction, "story": if fiction { json!("glass-city") } else { Value::Null }, "lang": ["en"]});
            return reply(&v.to_string(), vec![]);
        }
        let first = req.messages[0].text();
        let raw_path = field(&first, "path:").trim_end_matches(".md").to_owned();
        let cite = field(&first, "cite it as:").to_owned();
        let captured = field(&first, "captured_at:");
        let date = &captured[..10];
        let hhmm = &captured[11..16];
        let text = capture_text(&first);
        let story = field(&first, "story:");
        let res = results(req);
        let journal = format!("vaults/life/journal/{}/{date}.md", &date[..4]);
        let sara = if story.is_empty() { "vaults/life/people/sara.md".to_owned() } else { format!("vaults/stories/{story}/characters/sara.md") };
        let mentions_sara = text.contains("Sara") || text.contains("سارا");

        // Phase 1: read what exists.
        if res.is_empty() {
            let mut calls = vec![];
            if story.is_empty() {
                calls.push(("page_read", json!({"path": journal})));
            }
            if mentions_sara {
                calls.push(("page_read", json!({"path": sara})));
            }
            if calls.is_empty() {
                return reply("Nothing to file.", vec![]);
            }
            return reply("", calls);
        }
        // Stale hash → read again (a real model would do the same).
        if res.last().is_some_and(|r| r.contains("stale base_hash")) {
            return reply("", vec![("page_read", json!({"path": if res.last().unwrap().contains("people") || res.last().unwrap().contains("characters") { &sara } else { &journal }}))]);
        }
        let wrote_any = res.iter().any(|r| r.starts_with("created") || r.starts_with("edited"));
        if wrote_any && !res.last().unwrap().starts_with("ERROR") {
            return reply("Filed.", vec![]);
        }
        // Phase 2: write, using the latest read of each page.
        let latest = |p: &str| res.iter().rev().find(|r| r.contains(&format!("path: {p}\n")) || (r.starts_with("ERROR") && r.contains(p))).cloned();
        let mut calls = vec![];
        let sara_link = if story.is_empty() { "[[vaults/life/people/sara|Sara]]".to_owned() } else { format!("[[vaults/stories/{story}/characters/sara|Sara]]") };
        if story.is_empty() {
            let entry = format!("### {hhmm}\n{} {}", text.replace("Sara", &sara_link), cite);
            match latest(&journal).as_deref().and_then(hash_of) {
                Some(h) => calls.push(("page_edit", json!({"path": journal, "base_hash": h, "edits": [{"op": "append_to_section", "heading": "Entries", "text": entry}]}))),
                None => calls.push(("page_create", json!({"path": journal, "frontmatter": {"type": "journal-day", "title": {"en": format!("Journal · {date}"), "fa": format!("روزنوشت · {date}")}, "summary": format!("Journal for {date}.")}, "body": format!("# {date}\n\n## Entries\n{entry}\n")}))),
            }
        }
        if mentions_sara {
            let fact = format!("- {date}: {} {}", text, cite);
            match latest(&sara).as_deref().and_then(hash_of) {
                Some(h) => calls.push(("page_edit", json!({"path": sara, "base_hash": h, "edits": [{"op": "append_to_section", "heading": "Timeline", "text": fact}]}))),
                None => calls.push(("page_create", json!({"path": sara, "frontmatter": {"type": if story.is_empty() { "person" } else { "character" }, "title": {"en": "Sara", "fa": "سارا"}, "aliases": ["sara", "سارا"], "summary": "Sara."}, "body": format!("# Sara\n\n## Timeline\n{fact}\n")}))),
            }
        }
        if (text.contains("headache") || text.contains("سردرد")) && story.is_empty() {
            let profile = "vaults/health/symptoms-log.md";
            if !res.iter().any(|r| r.contains(profile)) {
                calls.push(("page_create", json!({"path": profile, "frontmatter": {"type": "symptom-log", "title": {"en": "Symptoms log", "fa": "گزارش علائم"}, "summary": "Symptoms the user mentioned."}, "body": "# Symptoms log\n"})));
            }
            calls.push(("claim_propose", json!({"page": profile, "text": format!("Had a headache on {date}"), "kind": "stated"})));
            calls.push(("claim_propose", json!({"page": profile, "text": "Headaches may follow nights of bad sleep", "kind": "inferred", "confidence": "low"})));
        }
        let _ = raw_path;
        reply("", calls)
    })
}
