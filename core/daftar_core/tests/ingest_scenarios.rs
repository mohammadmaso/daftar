//! M2 scenario tests (§13): 1 (full), 2, 6, 9, plus a Persian journal note.

mod common;

use common::archivist::{ai_config, archivist, runtime};
use common::{Device, Remote, no_conflict_markers};
use daftar_core::agent::Cancel;
use daftar_core::ledger;
use daftar_core::providers::{
    ChatRequest, ChatResponse, MockProvider, MsgRole, StopReason, ToolCall,
};
use daftar_core::queue::{JobKind, JobState};
use daftar_core::raw::{self, RawStatus};
use daftar_core::session::Session;
use daftar_core::sync::{GitAuth, SyncState};
use daftar_core::testutil::zoned;
use daftar_core::{pages, review, wiki};
use serde_json::json;

fn session(d: &Device) -> Session {
    let mut c = d.lib.config().unwrap();
    c.ai = ai_config();
    d.lib.save_config(&c).unwrap();
    Session::open(d.root()).unwrap()
}

fn sync(s: &Session) -> daftar_core::sync::SyncOutcome {
    s.sync(
        &GitAuth::None,
        &zoned("2026-09-23T21:00:00+03:30[Asia/Tehran]"),
    )
    .unwrap()
}

async fn run(
    s: &Session,
    rt: &daftar_core::runtime::AiRuntime,
) -> Vec<daftar_core::session::JobReport> {
    s.run_jobs(rt, true, &Cancel::default()).await.unwrap()
}

/// Scenario 1: 20 voice captures offline → transcribed, ingested, committed and pushed in order.
#[tokio::test(flavor = "multi_thread")]
async fn offline_voice_captures_are_filed_in_order() {
    let remote = Remote::new();
    let phone = Device::clone_from(&remote, "pixel-8");
    let s = session(&phone);
    let audio = phone.root().join("../rec.m4a");
    std::fs::write(&audio, b"audio").unwrap();
    remote.go_offline();
    for i in 0..20 {
        s.capture_voice(
            &audio,
            None,
            &zoned(&format!("2026-09-23T10:{i:02}:00+03:30[Asia/Tehran]")),
        )
        .unwrap();
        assert_eq!(sync(&s).state, SyncState::Offline);
    }
    // Offline: the AI queue waits without failing anything.
    let mock = archivist();
    let transcripts: Vec<String> = (0..20)
        .map(|i| format!("Note number {i:02} about Sara"))
        .collect();
    let rt = runtime(mock);
    assert!(
        s.run_jobs(&rt, false, &Cancel::default())
            .await
            .unwrap()
            .is_empty()
    );

    remote.go_online();
    // Transcripts come from a scripted STT; the archivist files each note.
    let script = daftar_core::providers::mock::Script {
        chat: vec![],
        transcripts,
    };
    let stt = MockProvider::new(script);
    let arch = archivist();
    // One provider for both roles: STT from the script, chat from the archivist.
    struct Both(MockProvider, MockProvider);
    #[async_trait::async_trait]
    impl daftar_core::providers::LlmProvider for Both {
        fn name(&self) -> &str {
            "Mock"
        }
        async fn chat(
            &self,
            r: &ChatRequest,
            d: daftar_core::providers::OnDelta<'_>,
        ) -> daftar_core::providers::ProviderResult<ChatResponse> {
            self.1.chat(r, d).await
        }
        async fn transcribe(
            &self,
            m: &str,
            a: Vec<u8>,
            f: &str,
            l: Option<&str>,
        ) -> daftar_core::providers::ProviderResult<String> {
            self.0.transcribe(m, a, f, l).await
        }
    }
    let rt = daftar_core::runtime::AiRuntime::new(ai_config(), Default::default())
        .with_provider("mock", std::sync::Arc::new(Both(stt, arch)));
    let reports = run(&s, &rt).await;
    assert_eq!(reports.len(), 40, "20 transcriptions + 20 ingests");
    assert!(
        reports.iter().all(|r| r.state == JobState::Done),
        "{reports:?}"
    );
    assert_eq!(sync(&s).state, SyncState::Synced);

    let subjects = remote.commit_subjects();
    let ingests: Vec<_> = subjects
        .iter()
        .filter(|s| s.starts_with("ingest:"))
        .collect();
    assert_eq!(ingests.len(), 20);
    // No duplicates and in capture order: the Sara timeline lists notes 00..19 in order.
    let laptop = Device::clone_from(&remote, "laptop");
    let page = laptop.read("vaults/life/people/sara.md");
    let order: Vec<usize> = (0..20)
        .map(|i| {
            page.find(&format!("Note number {i:02}"))
                .expect("each note filed once")
        })
        .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "filed in order");
    assert_eq!(page.matches("Note number 07").count(), 1);
    let live = ledger::live_ingests_by_source(&ledger::all(&laptop.lib).unwrap());
    assert_eq!(live.len(), 20);
    assert!(live.values().all(|ops| ops.len() == 1));
    // Generated index lists the page.
    assert!(
        laptop
            .read("vaults/life/index.md")
            .contains("[[vaults/life/people/sara|Sara · سارا]]")
    );
}

/// Scenario 2: two devices ingest different captures touching the same person page while offline.
#[tokio::test(flavor = "multi_thread")]
async fn divergent_ingests_on_the_same_page_are_replayed() {
    let remote = Remote::new();
    let a = Device::clone_from(&remote, "pixel-8");
    let b = Device::clone_from(&remote, "laptop");
    let sa = session(&a);
    sync(&sa);
    let sb = session(&b);
    sync(&sb);
    sync(&sa);

    sa.capture_text(
        "Sara called about the Isfahan trip",
        None,
        &zoned("2026-09-23T10:00:00+03:30[Asia/Tehran]"),
    )
    .unwrap();
    sb.capture_text(
        "Lunch with Sara, she got the new job",
        None,
        &zoned("2026-09-23T13:00:00+03:30[Asia/Tehran]"),
    )
    .unwrap();
    assert!(
        run(&sa, &runtime(archivist()))
            .await
            .iter()
            .all(|r| r.state == JobState::Done)
    );
    assert!(
        run(&sb, &runtime(archivist()))
            .await
            .iter()
            .all(|r| r.state == JobState::Done)
    );

    assert_eq!(sync(&sa).state, SyncState::Synced);
    let ob = sync(&sb);
    assert_eq!(ob.state, SyncState::Synced, "{ob:?}");
    assert_eq!(
        ob.replays.len(),
        1,
        "B's op conflicted and must be replayed: {ob:?}"
    );
    // The replay runs as a normal job on top of the merged state.
    let reports = run(&sb, &runtime(archivist())).await;
    assert!(
        reports
            .iter()
            .any(|r| r.kind == JobKind::Ingest && r.state == JobState::Done),
        "{reports:?}"
    );
    assert_eq!(sync(&sb).state, SyncState::Synced);
    sync(&sa);

    for d in [&a, &b] {
        let page = d.read("vaults/life/people/sara.md");
        assert!(
            page.contains("Isfahan trip") && page.contains("new job"),
            "{page}"
        );
        no_conflict_markers(d.root());
        assert!(d.read("vaults/life/index.md").contains("people/sara"));
    }
    let entries = ledger::all(&a.lib).unwrap();
    assert_eq!(
        entries.iter().filter(|e| e.replayed_from.is_some()).count(),
        1
    );
    let live = ledger::live_ingests_by_source(&entries);
    assert_eq!(live.len(), 2);
    assert!(live.values().all(|v| v.len() == 1));
}

/// Scenario 6: a fiction capture never produces claims about the user.
#[tokio::test(flavor = "multi_thread")]
async fn fiction_stays_in_its_story() {
    let remote = Remote::new();
    let d = Device::clone_from(&remote, "laptop");
    let s = session(&d);
    s.capture_text(
        "Story idea: in Glass City, Sara is diagnosed with diabetes.",
        None,
        &zoned("2026-09-23T10:00:00+03:30[Asia/Tehran]"),
    )
    .unwrap();
    // A "confused" model that tries to write a health claim about the user after the story pages.
    let confused = MockProvider::with_fn(|req: &ChatRequest| {
        if req.system.contains("You are the ROUTER") {
            return resp(
                r#"{"targets":[{"vault":"stories","reason":"story idea","confidence":0.9}],"is_fiction":true,"story":"glass-city","lang":["en"]}"#,
                vec![],
            );
        }
        let tools: Vec<String> = req
            .messages
            .iter()
            .filter(|m| m.role == MsgRole::Tool)
            .map(|m| m.text())
            .collect();
        match tools.len() {
            0 => resp(
                "",
                vec![
                    call(
                        "claim_propose",
                        json!({"page": "vaults/health/profile.md", "text": "Has diabetes", "kind": "stated"}),
                    ),
                    call(
                        "page_create",
                        json!({"path": "vaults/health/conditions/diabetes", "frontmatter": {"type": "condition", "title": {"en": "Diabetes", "fa": "دیابت"}, "summary": "x"}, "body": "x"}),
                    ),
                ],
            ),
            2 => resp(
                "",
                vec![call(
                    "page_create",
                    json!({
                        "path": "vaults/stories/glass-city/characters/sara",
                        "frontmatter": {"type": "character", "title": {"en": "Sara", "fa": "سارا"}, "summary": "Protagonist of Glass City."},
                        "body": format!("# Sara\n\n## Facts\n- Diagnosed with diabetes {}\n", first_field(req, "cite it as:"))
                    }),
                )],
            ),
            _ => resp("Filed the story idea.", vec![]),
        }
    });
    let reports = run(&s, &runtime(confused)).await;
    assert!(
        reports.iter().all(|r| r.state == JobState::Done),
        "{reports:?}"
    );
    assert!(
        d.lib
            .path("vaults/stories/glass-city/characters/sara.md")
            .exists()
    );
    assert!(!d.lib.path("vaults/health/profile.md").exists());
    assert!(!d.lib.path("vaults/health/conditions/diabetes.md").exists());
    let all = pages::load_all(&d.lib).unwrap();
    assert!(
        all.iter()
            .filter(|p| !p.path.starts_with("vaults/stories/"))
            .all(|p| wiki::claims(&p.body).is_empty())
    );
    assert!(
        !review::list(&d.lib)
            .unwrap()
            .iter()
            .any(|r| r.kind == review::ReviewKind::Claim)
    );
}

fn resp(text: &str, calls: Vec<ToolCall>) -> ChatResponse {
    ChatResponse {
        text: text.into(),
        stop: if calls.is_empty() {
            StopReason::EndTurn
        } else {
            StopReason::ToolUse
        },
        tool_calls: calls,
        usage: Default::default(),
    }
}

fn call(name: &str, args: serde_json::Value) -> ToolCall {
    ToolCall {
        id: format!("c-{name}"),
        name: name.into(),
        arguments: args,
    }
}

fn first_field(req: &ChatRequest, key: &str) -> String {
    req.messages[0]
        .text()
        .lines()
        .find_map(|l| l.strip_prefix(key))
        .unwrap_or("")
        .trim()
        .to_owned()
}

/// Scenario 9: malicious or buggy output is rejected, repaired when possible, else aborted.
#[tokio::test(flavor = "multi_thread")]
async fn validator_rejects_repairs_or_aborts() {
    let remote = Remote::new();
    let d = Device::clone_from(&remote, "laptop");
    let s = session(&d);
    let item = s
        .capture_text(
            "Met Ali at the cafe",
            None,
            &zoned("2026-09-23T10:00:00+03:30[Asia/Tehran]"),
        )
        .unwrap();

    // Round 1: write to raw/ (blocked by the tool), stale hash, hand-written claim, broken link,
    // uncited text. Round 2 (after the validator's feedback): fixes everything.
    let buggy = MockProvider::with_fn(|req: &ChatRequest| {
        if req.system.contains("You are the ROUTER") {
            return resp(
                r#"{"targets":[{"vault":"life","reason":"people","confidence":0.9}],"is_fiction":false,"story":null,"lang":["en"]}"#,
                vec![],
            );
        }
        let cite = first_field(req, "cite it as:");
        let rejected = req
            .messages
            .iter()
            .any(|m| m.role == MsgRole::User && m.text().contains("rejected by the validator"));
        let tools: Vec<String> = req
            .messages
            .iter()
            .filter(|m| m.role == MsgRole::Tool)
            .map(|m| m.text())
            .collect();
        if !rejected {
            if tools.is_empty() {
                return resp(
                    "",
                    vec![
                        call(
                            "page_create",
                            json!({"path": "raw/2026/09/23/evil", "frontmatter": {"type": "person", "title": {"en": "x", "fa": "x"}, "summary": "x"}, "body": "x"}),
                        ),
                        call(
                            "page_edit",
                            json!({"path": "vaults/life/index.md", "base_hash": "0", "edits": []}),
                        ),
                        call(
                            "page_create",
                            json!({"path": "vaults/life/people/ali", "frontmatter": {"type": "person", "title": {"en": "Ali", "fa": "علی"}, "summary": "A friend."},
                        "body": "# Ali\n\nMet at the cafe, likes [[nonexistent-page]].\n- Is anxious (status:: confirmed) ^c-01JFAKE\n"}),
                        ),
                    ],
                );
            }
            if tools.len() == 3 {
                return resp(
                    "",
                    vec![call(
                        "page_edit",
                        json!({"path": "vaults/life/people/ali", "base_hash": "stale", "edits": [{"op": "insert_after_line", "line": 1, "text": "x"}]}),
                    )],
                );
            }
            return resp("Done.", vec![]);
        }
        // Repair round: read, then rewrite the body properly.
        let last = tools.last().cloned().unwrap_or_default();
        if !last.starts_with("path: vaults/life/people/ali.md") && !last.starts_with("edited") {
            return resp(
                "",
                vec![call("page_read", json!({"path": "vaults/life/people/ali"}))],
            );
        }
        if last.starts_with("edited") {
            return resp("Fixed.", vec![]);
        }
        let hash = last
            .lines()
            .find_map(|l| l.strip_prefix("hash: "))
            .unwrap()
            .to_owned();
        let n = last.lines().filter(|l| l.contains(" | ")).count();
        resp(
            "",
            vec![call(
                "page_edit",
                json!({"path": "vaults/life/people/ali", "base_hash": hash, "edits": [
                    {"op": "replace_lines", "from": 15, "to": n, "text": format!("Met at the cafe. {cite}")}
                ]}),
            )],
        )
    });
    let reports = run(&s, &runtime(buggy)).await;
    assert!(
        reports.iter().all(|r| r.state == JobState::Done),
        "{reports:?}"
    );
    let page = d.read("vaults/life/people/ali.md");
    assert!(
        page.contains("Met at the cafe.")
            && !page.contains("nonexistent-page")
            && !page.contains("^c-01JFAKE"),
        "{page}"
    );
    assert!(!d.lib.path("raw/2026/09/23/evil.md").exists());
    assert!(!d.read("vaults/life/index.md").contains("evil"));
    assert_eq!(
        raw::read(&d.lib, &item.path).unwrap().meta.status,
        RawStatus::Ingested
    );

    // A model that never fixes its output: aborted, capture stays pending, reason is readable.
    let item2 = s
        .capture_text(
            "Another note",
            None,
            &zoned("2026-09-23T11:00:00+03:30[Asia/Tehran]"),
        )
        .unwrap();
    let stubborn = MockProvider::with_fn(|req: &ChatRequest| {
        if req.system.contains("You are the ROUTER") {
            return resp(
                r#"{"targets":[{"vault":"life","reason":"x","confidence":0.9}],"is_fiction":false}"#,
                vec![],
            );
        }
        let tools = req
            .messages
            .iter()
            .filter(|m| m.role == MsgRole::Tool)
            .count();
        if tools == 0 {
            return resp(
                "",
                vec![call(
                    "page_create",
                    json!({"path": "vaults/life/topics/uncited", "frontmatter": {"type": "topic", "title": {"en": "U", "fa": "یو"}, "summary": "u"}, "body": "An uncited claim."}),
                )],
            );
        }
        resp("Done.", vec![])
    });
    let reports = run(&s, &runtime(stubborn)).await;
    let r = reports.last().unwrap();
    assert_eq!(r.state, JobState::Failed);
    assert!(
        r.message
            .as_deref()
            .unwrap()
            .contains("without a source citation"),
        "{r:?}"
    );
    assert_eq!(
        raw::read(&d.lib, &item2.path).unwrap().meta.status,
        RawStatus::Pending
    );
    assert!(
        !d.lib.path("vaults/life/topics/uncited.md").exists(),
        "nothing half-applied"
    );
}

/// M2 acceptance: a spoken Persian journal note → journal day section, person link, proposed claim.
#[tokio::test(flavor = "multi_thread")]
async fn persian_voice_note_is_filed_with_journal_link_and_claim() {
    let remote = Remote::new();
    let d = Device::clone_from(&remote, "pixel-8");
    let s = session(&d);
    let audio = d.root().join("../fa.m4a");
    std::fs::write(&audio, b"audio").unwrap();
    s.capture_voice(
        &audio,
        None,
        &zoned("2026-09-23T08:12:00+03:30[Asia/Tehran]"),
    )
    .unwrap();

    struct Both(MockProvider);
    #[async_trait::async_trait]
    impl daftar_core::providers::LlmProvider for Both {
        fn name(&self) -> &str {
            "Mock"
        }
        async fn chat(
            &self,
            r: &ChatRequest,
            d: daftar_core::providers::OnDelta<'_>,
        ) -> daftar_core::providers::ProviderResult<ChatResponse> {
            self.0.chat(r, d).await
        }
        async fn transcribe(
            &self,
            _: &str,
            _: Vec<u8>,
            _: &str,
            _: Option<&str>,
        ) -> daftar_core::providers::ProviderResult<String> {
            Ok("دیشب بد خوابیدم و امروز صبح سردرد داشتم. سارا زنگ زد.".into())
        }
    }
    let rt = daftar_core::runtime::AiRuntime::new(ai_config(), Default::default())
        .with_provider("mock", std::sync::Arc::new(Both(archivist())));
    let reports = run(&s, &rt).await;
    assert!(
        reports.iter().all(|r| r.state == JobState::Done),
        "{reports:?}"
    );

    let journal = d.read("vaults/life/journal/2026/2026-09-23.md");
    assert!(
        journal.contains("### 08:12") && journal.contains("سردرد"),
        "{journal}"
    );
    assert!(
        journal.contains("[[vaults/life/people/sara|Sara]]") || journal.contains("سارا"),
        "{journal}"
    );
    assert!(
        journal.contains("[[raw/2026/09/23/"),
        "cites the capture: {journal}"
    );
    let log = d.read("vaults/health/symptoms-log.md");
    let claims = wiki::claims(&log);
    assert_eq!(claims.len(), 2, "{log}");
    assert_eq!(
        claims
            .iter()
            .find(|c| c.text.starts_with("Had a headache"))
            .unwrap()
            .status,
        "confirmed"
    );
    assert_eq!(
        claims
            .iter()
            .find(|c| c.text.starts_with("Headaches may follow"))
            .unwrap()
            .status,
        "proposed"
    );
    let items = review::list(&d.lib).unwrap();
    assert_eq!(
        items
            .iter()
            .filter(|r| r.kind == review::ReviewKind::Claim)
            .count(),
        1,
        "only the inferred claim needs review"
    );
    let e = ledger::all(&d.lib).unwrap().pop().unwrap();
    assert!(
        e.summary
            .starts_with("Filed your voice note to Health and Life")
            || e.summary
                .starts_with("Filed your voice note to Life and Health"),
        "{}",
        e.summary
    );
    assert!(d.read("log/2026-09.md").contains("ingest | health, life"));
}
