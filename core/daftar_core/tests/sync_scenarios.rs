//! Scenario tests from brief §13 that do not need an AI provider (M1).

mod common;

use common::{Device, Remote, no_conflict_markers};
use daftar_core::ledger;
use daftar_core::raw::RawStatus;
use daftar_core::review;
use daftar_core::sync::SyncState;

/// Scenario 1 (without AI): 20 offline captures are committed and pushed in order, no duplicates.
#[test]
fn offline_captures_sync_in_order() {
    let remote = Remote::new();
    let phone = Device::clone_from(&remote, "pixel-8");
    remote.go_offline();
    for i in 0..20 {
        phone.capture(
            &format!("note {i:02}"),
            &format!("2026-09-23T10:{i:02}:00+03:30[Asia/Tehran]"),
        );
        let o = phone.sync();
        assert_eq!(o.state, SyncState::Offline, "offline sync must not fail");
    }
    remote.go_online();
    let o = phone.sync();
    assert_eq!(o.state, SyncState::Synced, "{o:?}");

    let laptop = Device::clone_from(&remote, "laptop");
    assert_eq!(laptop.raw_files(), phone.raw_files());
    assert_eq!(laptop.raw_files().len(), 20);
    let items = daftar_core::raw::list_day(
        &laptop.lib,
        daftar_core::layout::Date {
            year: 2026,
            month: 9,
            day: 23,
        },
    )
    .unwrap();
    let texts: Vec<_> = items.iter().map(|i| i.body.clone()).collect();
    let expected: Vec<_> = (0..20).map(|i| format!("note {i:02}")).collect();
    assert_eq!(texts, expected, "captures keep their order");
    let subjects = remote.commit_subjects();
    assert_eq!(
        subjects
            .iter()
            .filter(|s| s.starts_with("capture:"))
            .count(),
        20
    );
}

/// Scenario 2, raw-only part: both devices capture while offline, then both sync.
#[test]
fn two_devices_offline_captures_merge_without_conflicts() {
    let remote = Remote::new();
    let a = Device::clone_from(&remote, "pixel-8");
    let b = Device::clone_from(&remote, "laptop");
    a.capture("A: Sara called", "2026-09-23T10:00:00+03:30[Asia/Tehran]");
    b.capture(
        "B: met Sara for lunch",
        "2026-09-23T12:00:00+03:30[Asia/Tehran]",
    );
    // Both append to the same monthly log: union merge must keep both lines.
    a.write("log/2026-09.md", "## [2026-09-23 10:00] capture | A\n");
    b.write("log/2026-09.md", "## [2026-09-23 12:00] capture | B\n");

    assert_eq!(a.sync().state, SyncState::Synced);
    let ob = b.sync();
    assert_eq!(ob.state, SyncState::Synced, "{ob:?}");
    assert!(ob.conflicts.is_empty(), "{ob:?}");
    assert_eq!(a.sync().state, SyncState::Synced);

    assert_eq!(a.raw_files(), b.raw_files());
    assert_eq!(a.raw_files().len(), 2);
    let log = b.read("log/2026-09.md");
    assert!(log.contains("| A") && log.contains("| B"), "{log}");
    assert_eq!(a.read("log/2026-09.md"), log);
    no_conflict_markers(a.root());
    no_conflict_markers(b.root());
}

/// Scenario 3: both devices ingest the same raw item; exactly one op survives.
#[test]
fn double_ingest_later_local_op_is_dropped() {
    let remote = Remote::new();
    let a = Device::clone_from(&remote, "pixel-8");
    let item = a.capture(
        "headache after bad sleep",
        "2026-09-23T10:00:00+03:30[Asia/Tehran]",
    );
    a.sync();
    let b = Device::clone_from(&remote, "laptop");

    a.fake_ingest(
        &item,
        "vaults/life/journal/2026/2026-09-23.md",
        "- headache (A)",
        "01K00000000000000000000001",
    );
    b.fake_ingest(
        &item,
        "vaults/life/journal/2026/2026-09-23.md",
        "- headache (B)",
        "01K00000000000000000000002",
    );
    assert_eq!(a.sync().state, SyncState::Synced);
    let ob = b.sync();
    assert_eq!(ob.state, SyncState::Synced);
    assert_eq!(
        ob.duplicates_dropped,
        vec!["01K00000000000000000000002".to_string()]
    );
    assert!(ob.replays.is_empty());

    for d in [&a, &b] {
        d.sync();
        let live = ledger::live_ingests_by_source(&ledger::all(&d.lib).unwrap());
        assert_eq!(
            live[&item.meta.id],
            vec!["01K00000000000000000000001".to_string()]
        );
        let page = d.read("vaults/life/journal/2026/2026-09-23.md");
        assert!(page.contains("(A)") && !page.contains("(B)"), "{page}");
    }
}

#[test]
fn double_ingest_earlier_local_op_wins_over_pushed_later_op() {
    let remote = Remote::new();
    let a = Device::clone_from(&remote, "pixel-8");
    let item = a.capture("headache", "2026-09-23T10:00:00+03:30[Asia/Tehran]");
    a.sync();
    let b = Device::clone_from(&remote, "laptop");

    // B's op is older but A pushes first.
    b.fake_ingest(
        &item,
        "vaults/life/journal/2026/2026-09-23.md",
        "- headache (B)",
        "01K00000000000000000000001",
    );
    a.fake_ingest(
        &item,
        "vaults/life/journal/2026/2026-09-23.md",
        "- headache (A)",
        "01K00000000000000000000002",
    );
    assert_eq!(a.sync().state, SyncState::Synced);
    let ob = b.sync();
    assert_eq!(ob.state, SyncState::Synced, "{ob:?}");
    assert!(
        ob.duplicates_dropped
            .contains(&"01K00000000000000000000002".to_string())
    );
    a.sync();
    for d in [&a, &b] {
        let live = ledger::live_ingests_by_source(&ledger::all(&d.lib).unwrap());
        assert_eq!(
            live[&item.meta.id],
            vec!["01K00000000000000000000001".to_string()]
        );
        let page = d.read("vaults/life/journal/2026/2026-09-23.md");
        assert!(page.contains("(B)") && !page.contains("(A)"), "{page}");
    }
}

/// An AI op whose page conflicts with remote work is dropped and returned for replay.
#[test]
fn conflicting_op_is_returned_for_replay() {
    let remote = Remote::new();
    let a = Device::clone_from(&remote, "pixel-8");
    let one = a.capture(
        "Sara is my cousin",
        "2026-09-23T10:00:00+03:30[Asia/Tehran]",
    );
    a.write("vaults/life/people/sara.md", "# Sara\n\nNotes.\n");
    a.sync();
    let b = Device::clone_from(&remote, "laptop");
    let two = b.capture(
        "Sara moved to Shiraz",
        "2026-09-23T11:00:00+03:30[Asia/Tehran]",
    );

    a.fake_ingest(
        &one,
        "vaults/life/people/sara.md",
        "- cousin",
        "01K00000000000000000000001",
    );
    b.sync(); // push B's capture first so it is not part of the conflict
    b.fake_ingest(
        &two,
        "vaults/life/people/sara.md",
        "- lives in Shiraz",
        "01K00000000000000000000002",
    );
    assert_eq!(a.sync().state, SyncState::Synced);
    let ob = b.sync();
    assert_eq!(ob.state, SyncState::Synced, "{ob:?}");
    assert_eq!(ob.replays.len(), 1, "{ob:?}");
    assert_eq!(ob.replays[0].op_id, "01K00000000000000000000002");
    assert_eq!(ob.replays[0].sources, vec![two.meta.id.clone()]);
    // The dropped op's effects are gone; the raw item is pending again, ready to be re-ingested.
    assert_eq!(
        daftar_core::raw::read(&b.lib, &two.path)
            .unwrap()
            .meta
            .status,
        RawStatus::Pending
    );
    no_conflict_markers(b.root());
}

/// Human edits on both sides of the same paragraph: no text lost, callout + review item.
#[test]
fn human_conflict_becomes_callout_and_review_item() {
    let remote = Remote::new();
    let a = Device::clone_from(&remote, "pixel-8");
    a.write(
        "vaults/life/people/sara.md",
        "# Sara\n\nSara lives in Tehran.\n\nOther notes.\n",
    );
    a.sync();
    let b = Device::clone_from(&remote, "laptop");

    a.write(
        "vaults/life/people/sara.md",
        "# Sara\n\nSara lives in Isfahan.\n\nOther notes.\n",
    );
    b.write(
        "vaults/life/people/sara.md",
        "# Sara\n\nSara lives in Shiraz now.\n\nOther notes.\n",
    );
    assert_eq!(a.sync().state, SyncState::Synced);
    let ob = b.sync();
    assert_eq!(ob.conflicts, vec!["vaults/life/people/sara.md".to_string()]);
    let page = b.read("vaults/life/people/sara.md");
    assert!(
        page.contains("Isfahan") && page.contains("Shiraz now"),
        "{page}"
    );
    assert!(page.contains("> [!conflict] From laptop"), "{page}");
    no_conflict_markers(b.root());
    let items = review::list(&b.lib).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, review::ReviewKind::SyncConflict);

    a.sync();
    assert_eq!(a.read("vaults/life/people/sara.md"), page);
}

/// Edits made outside the app (e.g. Obsidian) are committed as human edits on the next sync.
#[test]
fn external_edits_are_committed() {
    let remote = Remote::new();
    let a = Device::clone_from(&remote, "laptop");
    a.write("vaults/work/topics/rust.md", "# Rust\n");
    a.sync();
    let subjects = remote.commit_subjects();
    assert!(
        subjects
            .iter()
            .any(|s| s == "edit: vaults/work/topics/rust.md"),
        "{subjects:?}"
    );
    // Device registration is committed separately from human edits.
    assert!(
        subjects
            .iter()
            .any(|s| s == "sync: capture status and devices"),
        "{subjects:?}"
    );
}

/// Unsealed voice captures stay local until transcribed.
#[test]
fn unsealed_voice_capture_is_not_committed() {
    use daftar_core::queue::JobKind;
    use daftar_core::raw::{self, NewCapture, RawKind};
    use daftar_core::testutil::zoned;

    let remote = Remote::new();
    let a = Device::clone_from(&remote, "pixel-8");
    let v = raw::create(
        &a.lib,
        &a.dev,
        &zoned("2026-09-23T10:00:00+03:30[Asia/Tehran]"),
        RawKind::Voice,
        NewCapture::default(),
    )
    .unwrap();
    let job = a
        .queue
        .enqueue(
            JobKind::Transcribe,
            Some(v.id()),
            serde_json::json!({}),
            true,
        )
        .unwrap();
    a.sync();
    assert!(Device::clone_from(&remote, "x").raw_files().is_empty());

    raw::seal(&a.lib, &v.path, "Slept badly.", Some("mock/stt")).unwrap();
    a.queue.complete(&job.id).unwrap();
    a.sync();
    assert_eq!(Device::clone_from(&remote, "y").raw_files().len(), 1);
}

/// §12: capture-to-saved under 100 ms. The core part is one small atomic file write.
#[test]
fn capture_is_fast() {
    let remote = Remote::new();
    let a = Device::clone_from(&remote, "pixel-8");
    let s = daftar_core::session::Session::open(a.root()).unwrap();
    let now = daftar_core::testutil::zoned("2026-09-23T10:00:00+03:30[Asia/Tehran]");
    let mut worst = std::time::Duration::ZERO;
    for i in 0..50 {
        let t = std::time::Instant::now();
        s.capture_text(&format!("note {i}"), None, &now).unwrap();
        worst = worst.max(t.elapsed());
    }
    assert!(worst < std::time::Duration::from_millis(100), "worst capture took {worst:?}");
}
