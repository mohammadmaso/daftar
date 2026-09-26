//! The eval harness itself (§15), run on its fixture cases with the scripted archivist so the
//! structural checks are exercised on every build. Real providers: `daftar eval CONFIG`.

mod common;

use std::sync::Arc;

use common::archivist::{ai_config, archivist};
use daftar_core::eval::{load_cases, run_case};
use daftar_core::runtime::AiRuntime;

#[tokio::test(flavor = "multi_thread")]
async fn fixture_ingest_cases_pass_with_the_scripted_model() {
    let cases =
        load_cases(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/eval"))
            .unwrap();
    assert!(cases.len() >= 9);
    for name in ["persian-journal-note", "fiction-stays-in-its-story"] {
        let mut case = cases.iter().find(|c| c.name == name).unwrap().clone();
        // The scripted model only knows a few words; real models get the fixture's own phrasing.
        if name == "persian-journal-note" {
            case.captures[0].text =
                "با Sara حرف زدم، سفر اصفهان پنج‌شنبه است. از دیشب سردرد دارم.".into();
        } else {
            case.captures[0].text = case.captures[0].text.replace("Story", "story");
        }
        let case = &case;
        let rt = AiRuntime::new(ai_config(), Default::default())
            .with_provider("mock", Arc::new(archivist()));
        let dir = tempfile::tempdir().unwrap();
        let r = run_case(case, &rt, dir.path()).await.unwrap();
        assert!(r.passed, "{name}: {:?} {:?}", r.failures, r.notes);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn failing_expectations_are_reported() {
    let cases =
        load_cases(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/eval"))
            .unwrap();
    let mut case = cases
        .iter()
        .find(|c| c.name == "persian-journal-note")
        .unwrap()
        .clone();
    case.expect.vaults.push("work".into());
    case.expect.never_write.push("vaults/life/".into());
    let rt = AiRuntime::new(ai_config(), Default::default())
        .with_provider("mock", Arc::new(archivist()));
    let dir = tempfile::tempdir().unwrap();
    let r = run_case(&case, &rt, dir.path()).await.unwrap();
    assert!(!r.passed);
    assert!(
        r.failures.iter().any(|f| f.contains("work vault")),
        "{:?}",
        r.failures
    );
    assert!(
        r.failures.iter().any(|f| f.contains("under vaults/life/")),
        "{:?}",
        r.failures
    );
}
