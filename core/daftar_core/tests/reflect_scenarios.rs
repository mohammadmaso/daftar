//! M8 scenarios: daily and weekly reflections (cited, once per day/week across devices, pattern
//! claims need three captures, crisis handling), and lint (verified quotes only, orphans, no
//! duplicate cards).

mod common;

use std::sync::Arc;

use common::archivist::{ai_config, archivist};
use common::{Device, Remote};
use daftar_core::agent::Cancel;
use daftar_core::ledger::{self, OpType, Usage};
use daftar_core::providers::{
    ChatRequest, ChatResponse, LlmProvider, MockProvider, MsgRole, StopReason, ToolCall,
};
use daftar_core::queue::JobState;
use daftar_core::reflect::{self, Due};
use daftar_core::review::{self, ReviewKind};
use daftar_core::runtime::AiRuntime;
use daftar_core::session::Session;
use daftar_core::testutil::zoned;
use serde_json::{Value, json};

fn session(d: &Device) -> Session {
    let mut c = d.lib.config().unwrap();
    c.ai = ai_config();
    d.lib.save_config(&c).unwrap();
    Session::open(d.root()).unwrap()
}

fn reply(text: &str, calls: Vec<(&str, Value)>) -> ChatResponse {
    ChatResponse {
        text: text.into(),
        stop: if calls.is_empty() {
            StopReason::EndTurn
        } else {
            StopReason::ToolUse
        },
        tool_calls: calls
            .into_iter()
            .enumerate()
            .map(|(i, (n, a))| ToolCall {
                id: format!("r{i}"),
                name: n.into(),
                arguments: a,
            })
            .collect(),
        usage: Usage::default(),
    }
}

fn raw_ids(text: &str) -> Vec<String> {
    let mut out: Vec<String> = text
        .split("[[raw/")
        .skip(1)
        .filter_map(|s| s.split(['|', ']']).next())
        .filter_map(|p| p.rsplit('-').next().map(str::to_owned))
        .collect();
    out.dedup();
    out
}

/// Files notes like the archivist; writes reflections and lint findings like a careful model.
fn reflector() -> MockProvider {
    let arch = archivist();
    MockProvider::with_fn(move |req: &ChatRequest| {
        let first = req.messages.first().map(|m| m.text()).unwrap_or_default();
        let tools: Vec<String> = req
            .messages
            .iter()
            .filter(|m| m.role == MsgRole::Tool)
            .map(|m| m.text())
            .collect();
        let last_user = req
            .messages
            .iter()
            .rev()
            .find(|m| m.role == MsgRole::User)
            .map(|m| m.text())
            .unwrap_or_default();
        if first.starts_with("DAILY REFLECT") {
            let crisis = req.system.contains("want to die");
            let cite = req
                .system
                .split("<day>")
                .nth(1)
                .unwrap_or_default()
                .split("[[raw/")
                .nth(1)
                .and_then(|s| s.split("]]").next())
                .map(|s| format!("[[raw/{s}]]"))
                .unwrap_or_default();
            return reply(
                &json!({
                    "summary": format!("Sara called about the trip and a headache followed a short night {cite}."),
                    "notification": "Your day, filed.\n2 notes, 1 person.\nextra line\nand another",
                    "care": crisis
                })
                .to_string(),
                vec![],
            );
        }
        if first.starts_with("LINT task") {
            return reply(
                &json!({"findings": [
                    {"kind": "contradiction", "summary": "Two vitamin D doses.", "fix": "Keep the newer one.",
                     "quotes": [{"path": "vaults/health/profile.md", "text": "Takes vitamin D 50,000 IU weekly"},
                                {"path": "vaults/health/labs/vit-d.md", "text": "Takes 1,000 IU vitamin D daily"}]},
                    {"kind": "stale", "summary": "Invented.", "quotes": [{"path": "vaults/health/profile.md", "text": "Takes iron supplements every day"}]}
                ]})
                .to_string(),
                vec![],
            );
        }
        if first.starts_with("WEEKLY REFLECT") {
            let page = first
                .lines()
                .find_map(|l| l.strip_prefix("Write the review to `"))
                .unwrap()
                .trim_end_matches("`.")
                .to_owned();
            let journals: Vec<String> = first
                .lines()
                .filter_map(|l| l.strip_prefix("- "))
                .map(str::to_owned)
                .collect();
            if tools.is_empty() {
                return reply(
                    "",
                    journals
                        .iter()
                        .map(|j| ("page_read", json!({"path": j})))
                        .collect(),
                );
            }
            if last_user.contains("patterns need at least three") {
                let read = tools
                    .iter()
                    .rev()
                    .find(|t| t.starts_with("path: vaults/mind/patterns/"));
                return match read {
                    None => reply(
                        "",
                        vec![(
                            "page_read",
                            json!({"path": "vaults/mind/patterns/short-nights.md"}),
                        )],
                    ),
                    Some(r) => {
                        let hash = r.lines().nth(1).unwrap().trim_start_matches("hash: ");
                        let Some(line) = r
                            .lines()
                            .find(|l| l.contains("^c-"))
                            .and_then(|l| l.split(" | ").next())
                            .and_then(|n| n.trim().parse::<usize>().ok())
                        else {
                            return reply("Removed the unsupported pattern.", vec![]);
                        };
                        reply(
                            "",
                            vec![(
                                "page_edit",
                                json!({"path": "vaults/mind/patterns/short-nights.md", "base_hash": hash, "edits": [{"op": "replace_lines", "from": line, "to": line, "text": ""}]}),
                            )],
                        )
                    }
                };
            }
            if tools.iter().any(|t| t.starts_with("edited"))
                || tools.iter().any(|t| t.contains("claim c-")) && !last_user.contains("rejected")
            {
                return reply("Wrote the weekly review.", vec![]);
            }
            let ids: Vec<String> = tools.iter().flat_map(|t| raw_ids(t)).collect();
            let links: Vec<String> = tools
                .iter()
                .flat_map(|t| {
                    t.split("[[raw/")
                        .skip(1)
                        .filter_map(|s| s.split(']').next())
                        .map(|s| format!("[[raw/{s}]]"))
                        .collect::<Vec<_>>()
                })
                .collect();
            let bullets = links
                .iter()
                .take(3)
                .map(|l| format!("- A note from the week {l}"))
                .collect::<Vec<_>>()
                .join("\n");
            return reply(
                "",
                vec![
                    (
                        "page_create",
                        json!({"path": page, "frontmatter": {"type": "review", "title": {"en": "Weekly review", "fa": "مرور هفتگی"}, "summary": "The week in review."}, "body": format!("# Week\n\n## What happened\n{bullets}\n")}),
                    ),
                    (
                        "page_create",
                        json!({"path": "vaults/mind/patterns/short-nights", "frontmatter": {"type": "pattern", "title": {"en": "Short nights", "fa": "شب‌های کوتاه"}, "summary": "Sleep and headaches."}, "body": "# Short nights
"}),
                    ),
                    (
                        "claim_propose",
                        json!({"page": "vaults/mind/patterns/short-nights", "text": "Headaches follow short nights", "kind": "inferred", "confidence": "low", "sources": ids.iter().take(2).collect::<Vec<_>>()}),
                    ),
                ],
            );
        }
        futures::executor::block_on(arch.chat(req, None)).unwrap()
    })
}

fn runtime() -> AiRuntime {
    let mut cfg = ai_config();
    cfg.roles.push(daftar_core::providers::RoleConfig {
        role: daftar_core::providers::Role::Reflect,
        provider: "mock".into(),
        model: "mock-large".into(),
        params: Default::default(),
    });
    AiRuntime::new(cfg, Default::default()).with_provider("mock", Arc::new(reflector()))
}

async fn run(s: &Session) -> Vec<daftar_core::session::JobReport> {
    s.run_jobs(&runtime(), true, &Cancel::default())
        .await
        .unwrap()
}

fn capture(s: &Session, text: &str, at: &str) {
    s.capture_text(text, None, &zoned(at)).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn daily_summary_once_with_neutral_notification() {
    let remote = Remote::new();
    let d = Device::clone_from(&remote, "pixel-8");
    let s = session(&d);
    capture(
        &s,
        "Sara called about the Isfahan trip",
        "2026-09-23T10:00:00+03:30[Asia/Tehran]",
    );
    capture(
        &s,
        "Bad sleep and a headache all morning",
        "2026-09-23T12:00:00+03:30[Asia/Tehran]",
    );
    assert!(run(&s).await.iter().all(|r| r.state == JobState::Done));

    let evening = zoned("2026-09-23T22:00:00+03:30[Asia/Tehran]");
    assert!(s.schedule_reflections(&evening).unwrap() >= 1);
    assert_eq!(
        s.schedule_reflections(&evening).unwrap(),
        0,
        "not queued twice"
    );
    let reports = run(&s).await;
    assert!(
        reports.iter().all(|r| r.state == JobState::Done),
        "{reports:?}"
    );
    let day = d.read("vaults/life/journal/2026/2026-09-23.md");
    assert!(
        day.contains("## Day summary\nSara called about the trip"),
        "{day}"
    );
    let (notes, help) = s.take_reflect_signals();
    assert_eq!(
        notes[0], "Your day, filed.\n2 notes, 1 person.\nextra line",
        "at most three lines"
    );
    assert!(!help);
    let entries = ledger::all(&d.lib).unwrap();
    assert!(
        entries
            .iter()
            .any(|e| e.op_type == OpType::Reflect && e.note.as_deref() == Some("daily:2026-09-23"))
    );
    let still_due = reflect::due(&d.lib, &Default::default(), &evening).unwrap();
    assert!(
        !still_due.contains(&Due::Daily {
            date: "2026-09-23".into()
        }),
        "done once for every device"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_heavy_day_gets_care_not_details() {
    let remote = Remote::new();
    let d = Device::clone_from(&remote, "pixel-8");
    let s = session(&d);
    capture(
        &s,
        "I want to die some days. Sara called.",
        "2026-09-23T10:00:00+03:30[Asia/Tehran]",
    );
    run(&s).await;
    let out = reflect::daily(
        &d.lib,
        s.device(),
        &runtime(),
        "2026-09-23".parse().unwrap(),
        true,
        &zoned("2026-09-23T22:00:00+03:30[Asia/Tehran]"),
        &std::sync::Mutex::new(()),
    )
    .await
    .unwrap();
    assert!(out.needs_help);
    assert_eq!(out.notification.as_deref(), Some("Your day, filed."));
}

#[tokio::test(flavor = "multi_thread")]
async fn weekly_review_is_cited_and_patterns_need_three_captures() {
    let remote = Remote::new();
    let d = Device::clone_from(&remote, "pixel-8");
    let s = session(&d);
    for (i, t) in [
        "Sara called about the trip",
        "Bad sleep and a headache",
        "Short night, headache again",
        "Lunch with Sara",
    ]
    .iter()
    .enumerate()
    {
        capture(
            &s,
            t,
            &format!("2026-09-1{}T10:00:00+03:30[Asia/Tehran]", 4 + i),
        );
    }
    assert!(run(&s).await.iter().all(|r| r.state == JobState::Done));
    let out = reflect::weekly(
        &d.lib,
        s.device(),
        &runtime(),
        "2026-W38",
        "2026-09-12".parse().unwrap(),
        "2026-09-18".parse().unwrap(),
        true,
        &zoned("2026-09-18T19:00:00+03:30[Asia/Tehran]"),
        &Cancel::default(),
        &std::sync::Mutex::new(()),
    )
    .await
    .unwrap();
    assert_eq!(out.page.as_deref(), Some("vaults/life/reviews/2026-w38.md"));
    let review = d.read("vaults/life/reviews/2026-w38.md");
    assert!(
        review.contains("## What happened") && review.matches("[[raw/").count() >= 3,
        "{review}"
    );
    let pattern = d.read("vaults/mind/patterns/short-nights.md");
    assert!(
        !pattern.contains("(status:: proposed)"),
        "a 2-source pattern claim is refused: {pattern}"
    );
    assert!(review::list(&d.lib).unwrap().iter().all(|r| {
        r.kind != ReviewKind::Claim
            || r.payload["page"] != "vaults/mind/patterns/short-nights.md"
            || d.read("vaults/mind/patterns/short-nights.md")
                .contains(r.payload["claim_id"].as_str().unwrap())
    }));
}

#[tokio::test(flavor = "multi_thread")]
async fn lint_surfaces_verified_findings_once() {
    let remote = Remote::new();
    let d = Device::clone_from(&remote, "laptop");
    let s = session(&d);
    let page = |path: &str, kind: &str, body: &str| {
        d.write(path, &format!("---\ntype: {kind}\ntitle: {{ en: \"T\", fa: \"ت\" }}\nsummary: \"s\"\n---\n\n{body}\n"));
    };
    page(
        "vaults/health/profile.md",
        "profile",
        "- Takes vitamin D 50,000 IU weekly (status:: confirmed) ^c-1\n\nSee [[labs/vit-d]].",
    );
    page(
        "vaults/health/labs/vit-d.md",
        "lab",
        "- Takes 1,000 IU vitamin D daily (status:: confirmed) ^c-2",
    );
    page(
        "vaults/life/ideas/forgotten.md",
        "idea",
        "An idea nobody links to.",
    );
    s.schedule_lint(true).unwrap();
    let reports = run(&s).await;
    assert!(
        reports.iter().all(|r| r.state == JobState::Done),
        "{reports:?}"
    );
    let cards: Vec<_> = review::list(&d.lib)
        .unwrap()
        .into_iter()
        .filter(|r| r.kind == ReviewKind::Lint)
        .collect();
    let kinds: Vec<&str> = cards
        .iter()
        .filter_map(|c| c.payload["kind"].as_str())
        .collect();
    assert!(kinds.contains(&"contradiction"), "{kinds:?}");
    assert!(
        !kinds.contains(&"stale"),
        "the invented quote is dropped: {kinds:?}"
    );
    assert!(cards.iter().any(|c| c.payload["kind"] == "orphan"
        && c.payload["paths"][0] == "vaults/life/ideas/forgotten.md"));
    let c = cards
        .iter()
        .find(|c| c.payload["kind"] == "contradiction")
        .unwrap();
    assert_eq!(c.payload["quotes"][0]["line"], 7);

    s.schedule_lint(true).unwrap();
    run(&s).await;
    let again = review::list(&d.lib)
        .unwrap()
        .into_iter()
        .filter(|r| r.kind == ReviewKind::Lint)
        .count();
    assert_eq!(again, cards.len(), "the same findings are not queued twice");
    assert!(
        ledger::all(&d.lib)
            .unwrap()
            .iter()
            .any(|e| e.op_type == OpType::Lint)
    );
}
