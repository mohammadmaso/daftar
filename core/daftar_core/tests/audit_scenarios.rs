//! M4 scenarios (§13): 5 (undo a middle op after later ops touched the same pages), plus clean
//! undo, undo of an undo, move to vault, and Review confirm / reject.

mod common;

use std::sync::Arc;

use common::archivist::{ai_config, archivist};
use common::{Device, Remote, no_conflict_markers};
use daftar_core::agent::Cancel;
use daftar_core::ledger::{self, OpType, Usage};
use daftar_core::ops::IngestOptions;
use daftar_core::providers::{
    ChatRequest, ChatResponse, LlmProvider, MockProvider, MsgRole, StopReason, ToolCall,
};
use daftar_core::queue::JobState;
use daftar_core::raw::{self, RawStatus};
use daftar_core::review_ops::Resolution;
use daftar_core::runtime::AiRuntime;
use daftar_core::session::{Session, UndoState};
use daftar_core::testutil::zoned;
use serde_json::{Value, json};

fn session(d: &Device) -> Session {
    let mut c = d.lib.config().unwrap();
    c.ai = ai_config();
    d.lib.save_config(&c).unwrap();
    Session::open(d.root()).unwrap()
}

fn reply(text: &str, calls: Vec<(&str, Value)>) -> ChatResponse {
    let tool_calls: Vec<ToolCall> = calls
        .into_iter()
        .enumerate()
        .map(|(i, (n, a))| ToolCall {
            id: format!("k{i}"),
            name: n.into(),
            arguments: a,
        })
        .collect();
    ChatResponse {
        text: text.into(),
        stop: if tool_calls.is_empty() {
            StopReason::EndTurn
        } else {
            StopReason::ToolUse
        },
        tool_calls,
        usage: Usage::default(),
    }
}

/// Archivist for ingest; for compensation it reads every touched page and deletes the lines that
/// cite the undone source (and a journal heading left empty by that), like the prompt asks.
fn archivist_and_compensator() -> MockProvider {
    let arch = archivist();
    MockProvider::with_fn(move |req: &ChatRequest| {
        if !req.system.contains("You undo one earlier operation") {
            return futures::executor::block_on(arch.chat(req, None)).unwrap();
        }
        let first = req.messages[0].text();
        let source = first
            .lines()
            .find_map(|l| l.strip_prefix("source path: "))
            .unwrap()
            .trim_end_matches(".md")
            .to_owned();
        let pages: Vec<String> = first
            .lines()
            .filter_map(|l| l.strip_prefix("- "))
            .map(str::to_owned)
            .collect();
        let results: Vec<String> = req
            .messages
            .iter()
            .filter(|m| m.role == MsgRole::Tool)
            .map(|m| m.text())
            .collect();
        if results.is_empty() {
            return reply(
                "",
                pages
                    .iter()
                    .map(|p| ("page_read", json!({"path": p})))
                    .collect(),
            );
        }
        if results.iter().any(|r| r.starts_with("edited")) {
            return reply("Removed the undone note.", vec![]);
        }
        let mut calls = vec![];
        for r in results.iter().filter(|r| r.starts_with("path: ")) {
            let path = r.lines().next().unwrap().trim_start_matches("path: ");
            let hash = r.lines().nth(1).unwrap().trim_start_matches("hash: ");
            let numbered: Vec<(usize, String)> = r
                .lines()
                .filter_map(|l| {
                    let (n, t) = l.split_once(" | ")?;
                    Some((n.trim().parse().ok()?, t.to_owned()))
                })
                .collect();
            let mut edits = vec![];
            for (i, (n, t)) in numbered.iter().enumerate() {
                if !t.contains(&source) {
                    continue;
                }
                let heading_before = i > 0 && numbered[i - 1].1.starts_with("### ");
                let from = if heading_before { n - 1 } else { *n };
                edits.push(json!({"op": "replace_lines", "from": from, "to": n, "text": ""}));
            }
            if !edits.is_empty() {
                calls.push((
                    "page_edit",
                    json!({"path": path, "base_hash": hash, "edits": edits}),
                ));
            }
        }
        reply("", calls)
    })
}

fn runtime() -> AiRuntime {
    AiRuntime::new(ai_config(), Default::default())
        .with_provider("mock", Arc::new(archivist_and_compensator()))
}

async fn run_all(s: &Session) {
    let reports = s
        .run_jobs(&runtime(), true, &Cancel::default())
        .await
        .unwrap();
    assert!(
        reports.iter().all(|r| r.state == JobState::Done),
        "{reports:?}"
    );
}

/// Files three Sara notes; returns (session, device, [op ids], [raw paths]).
async fn three_notes(remote: &Remote) -> (Session, Device, Vec<String>, Vec<String>) {
    let d = Device::clone_from(remote, "laptop");
    let s = session(&d);
    let mut raws = vec![];
    for (i, t) in [
        "Sara called about the Isfahan trip",
        "Sara said she is moving to Shiraz",
        "Lunch with Sara, she got the new job",
    ]
    .iter()
    .enumerate()
    {
        let item = s
            .capture_text(
                t,
                None,
                &zoned(&format!("2026-09-23T1{i}:00:00+03:30[Asia/Tehran]")),
            )
            .unwrap();
        raws.push(item.path);
        run_all(&s).await;
    }
    let ops = ledger::all(&d.lib)
        .unwrap()
        .into_iter()
        .filter(|e| e.op_type == OpType::Ingest)
        .map(|e| e.op_id)
        .collect::<Vec<_>>();
    assert_eq!(ops.len(), 3);
    (s, d, ops, raws)
}

/// Scenario 5: undoing a middle op after later ops touched the same pages removes only that
/// source's contributions (compensating op), keeps everything else, and marks the capture excluded.
#[tokio::test(flavor = "multi_thread")]
async fn undo_middle_op_compensates() {
    let remote = Remote::new();
    let (s, d, ops, raws) = three_notes(&remote).await;
    let state = s.undo(&ops[1], None).unwrap();
    assert_eq!(
        state,
        UndoState::Queued,
        "later ops overlap: a plain revert must not apply"
    );
    run_all(&s).await;

    for page in [
        "vaults/life/people/sara.md",
        "vaults/life/journal/2026/2026-09-23.md",
    ] {
        let text = d.read(page);
        assert!(text.contains("Isfahan"), "{page}: first note kept\n{text}");
        assert!(text.contains("new job"), "{page}: third note kept\n{text}");
        assert!(
            !text.contains("Shiraz"),
            "{page}: undone note removed\n{text}"
        );
        assert!(
            !text.contains(raws[1].trim_end_matches(".md")),
            "{page}: citation removed"
        );
    }
    no_conflict_markers(d.root());
    let entries = ledger::all(&d.lib).unwrap();
    let comp = entries
        .iter()
        .find(|e| e.op_type == OpType::Compensate)
        .unwrap();
    assert_eq!(comp.reverts.as_deref(), Some(ops[1].as_str()));
    assert!(ledger::reverted_ops(&entries).contains(&ops[1]));
    assert_eq!(
        raw::read(&d.lib, &raws[1]).unwrap().meta.status,
        RawStatus::Excluded
    );
    let sara = daftar_core::pages::read(&d.lib, "vaults/life/people/sara.md").unwrap();
    let undone_id = raw::read(&d.lib, &raws[1]).unwrap().meta.id;
    assert!(
        !sara.meta.sources.contains(&undone_id),
        "source dropped from frontmatter"
    );
    let activity = s.activity(10, None).unwrap();
    assert_eq!(activity[0].op_type, OpType::Compensate);
    assert!(
        activity
            .iter()
            .find(|o| o.op_id == ops[1])
            .unwrap()
            .reverted
    );
}

/// Undo of the latest op is a clean revert; undoing that undo brings the note back.
#[tokio::test(flavor = "multi_thread")]
async fn undo_and_redo_by_revert() {
    let remote = Remote::new();
    let (s, d, ops, raws) = three_notes(&remote).await;
    let diff = s.op_diff(&ops[2]).unwrap();
    assert!(diff.iter().any(|f| f.path == "vaults/life/people/sara.md"
        && f.lines.iter().any(|l| l.text.contains("new job"))));

    let UndoState::Done(undo) = s.undo(&ops[2], None).unwrap() else {
        panic!("latest op should revert cleanly");
    };
    assert!(!d.read("vaults/life/people/sara.md").contains("new job"));
    assert!(d.read("vaults/life/people/sara.md").contains("Shiraz"));
    assert_eq!(
        raw::read(&d.lib, &raws[2]).unwrap().meta.status,
        RawStatus::Excluded
    );
    assert!(
        d.read("vaults/life/index.md").contains("Sara"),
        "indexes regenerated"
    );
    assert!(
        d.read(&format!("log/2026-{:02}.md", jiff::Zoned::now().month()))
            .contains("revert-op"),
        "the undo is logged"
    );

    let UndoState::Done(_) = s.undo(&undo, None).unwrap() else {
        panic!("undo of undo reverts cleanly");
    };
    assert!(d.read("vaults/life/people/sara.md").contains("new job"));
    assert_eq!(
        raw::read(&d.lib, &raws[2]).unwrap().meta.status,
        RawStatus::Ingested
    );
    let entries = ledger::all(&d.lib).unwrap();
    assert!(!ledger::reverted_ops(&entries).contains(&ops[2]));
    assert!(
        s.undo(&ops[0], None).is_ok(),
        "first op can still be undone"
    );
    no_conflict_markers(d.root());
}

/// Move to vault: undo, then file again with the vault forced.
#[tokio::test(flavor = "multi_thread")]
async fn move_to_vault_refiles() {
    let remote = Remote::new();
    let (s, d, ops, raws) = three_notes(&remote).await;
    let opts = IngestOptions {
        forced_vault: Some("work".into()),
        ..Default::default()
    };
    s.undo(&ops[2], Some(opts)).unwrap();
    assert_eq!(
        raw::read(&d.lib, &raws[2]).unwrap().meta.status,
        RawStatus::Pending
    );
    run_all(&s).await;
    let item = raw::read(&d.lib, &raws[2]).unwrap();
    assert_eq!(item.meta.status, RawStatus::Ingested);
    let live = ledger::live_ingests_by_source(&ledger::all(&d.lib).unwrap());
    let new_op = live[&item.meta.id].last().unwrap();
    assert_ne!(new_op, &ops[2]);
    let e = s.op(new_op).unwrap();
    assert_eq!(e.forced_vault.as_deref(), Some("work"));
}

/// Review: confirming and rejecting proposed claims edits the claim lines, removes the cards, and
/// a rejected claim is passed to later filings of the same capture.
#[tokio::test(flavor = "multi_thread")]
async fn review_confirm_and_reject_claims() {
    let remote = Remote::new();
    let d = Device::clone_from(&remote, "laptop");
    let s = session(&d);
    let item = s
        .capture_text(
            "Bad sleep and a headache all morning",
            None,
            &zoned("2026-09-23T10:00:00+03:30[Asia/Tehran]"),
        )
        .unwrap();
    run_all(&s).await;
    s.capture_text(
        "Another headache after a short night",
        None,
        &zoned("2026-09-23T18:00:00+03:30[Asia/Tehran]"),
    )
    .unwrap();
    run_all(&s).await;
    let log = d.read("vaults/health/symptoms-log.md");
    assert!(
        log.contains("Had a headache on 2026-09-23 (status:: confirmed)"),
        "stated facts are confirmed without review: {log}"
    );
    let cards = s.review_cards().unwrap();
    assert_eq!(cards.len(), 2, "one inferred claim per note: {cards:?}");
    let (first, second) = (&cards[0], &cards[1]);
    assert_eq!(first.claim.as_ref().unwrap().status, "proposed");

    s.resolve_review(
        &first.item.id,
        Resolution::ConfirmClaim {
            text: Some("Headaches often follow short nights".into()),
        },
    )
    .unwrap();
    s.resolve_review(&second.item.id, Resolution::RejectClaim)
        .unwrap();

    let log = d.read("vaults/health/symptoms-log.md");
    assert!(
        log.contains("- Headaches often follow short nights (status:: confirmed) (src:: "),
        "{log}"
    );
    assert!(
        !log.contains("(confidence:: low)"),
        "confirmed claims drop the confidence: {log}"
    );
    assert_eq!(log.matches("may follow").count(), 0, "{log}");
    assert!(s.review_cards().unwrap().is_empty());
    assert!(
        s.resolve_review(&second.item.id, Resolution::RejectClaim)
            .is_err()
    );

    let entries = ledger::all(&d.lib).unwrap();
    assert_eq!(
        entries
            .iter()
            .filter(|e| e.op_type == OpType::Review)
            .count(),
        2
    );
    let rejected =
        ledger::rejected_for_source(&entries, &second.claim.as_ref().unwrap().sources[0]);
    assert_eq!(rejected.len(), 1);
    assert!(rejected[0].text.contains("may follow"));
    let _ = item;
    no_conflict_markers(d.root());
}
