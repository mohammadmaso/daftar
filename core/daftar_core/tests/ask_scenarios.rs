//! M5 scenarios: answers cite real pages, absent facts are not invented, scopes isolate stories,
//! answers can be saved to the wiki, drafts never overwrite chapters.

mod common;

use std::sync::{Arc, Mutex};

use common::archivist::{ai_config, archivist};
use common::{Device, Remote};
use daftar_core::agent::Cancel;
use daftar_core::ask::AskScope;
use daftar_core::ledger::Usage;
use daftar_core::providers::{
    ChatRequest, ChatResponse, MockProvider, MsgRole, Part, StopReason, ToolCall,
};
use daftar_core::raw::{self, RawKind, RawStatus};
use daftar_core::runtime::AiRuntime;
use daftar_core::session::Session;
use serde_json::{Value, json};

fn session(d: &Device) -> Session {
    let mut c = d.lib.config().unwrap();
    c.ai = ai_config();
    d.lib.save_config(&c).unwrap();
    Session::open(d.root()).unwrap()
}

fn page(d: &Device, path: &str, kind: &str, en: &str, fa: &str, body: &str) {
    d.write(
        path,
        &format!(
            "---\ntype: {kind}\ntitle: {{ en: \"{en}\", fa: \"{fa}\" }}\nsummary: \"{en}\"\n---\n\n{body}\n"
        ),
    );
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
                id: format!("q{i}"),
                name: n.into(),
                arguments: a,
            })
            .collect(),
        usage: Usage::default(),
    }
}

/// Searches with the question's key word, reads the first hit (and, to probe scoping, Sara's
/// personal page), then answers only from what the tools returned.
fn librarian(saw_image: Arc<Mutex<bool>>) -> MockProvider {
    MockProvider::with_fn(move |req: &ChatRequest| {
        let q = req
            .messages
            .last()
            .filter(|m| m.role == MsgRole::User)
            .map(|m| m.text());
        let tools: Vec<String> = req
            .messages
            .iter()
            .filter(|m| m.role == MsgRole::Tool)
            .map(|m| m.text())
            .collect();
        if let Some(q) = q {
            if req
                .messages
                .last()
                .unwrap()
                .parts
                .iter()
                .any(|p| matches!(p, Part::Image { .. }))
            {
                *saw_image.lock().unwrap() = true;
            }
            let word = ["vitamin", "passport", "diabetes", "Sara"]
                .into_iter()
                .find(|w| q.contains(w))
                .unwrap_or("x");
            return reply(
                "",
                vec![
                    ("search", json!({"query": word})),
                    ("page_read", json!({"path": "vaults/life/people/sara.md"})),
                ],
            );
        }
        let hits = &tools[0];
        let first_hit = hits
            .lines()
            .find_map(|l| l.strip_prefix("- "))
            .and_then(|l| l.split(" | ").next())
            .map(str::to_owned);
        if tools.len() == 2 {
            if let Some(p) = &first_hit {
                return reply("", vec![("page_read", json!({"path": p}))]);
            }
            return reply(
                "The wiki doesn't say anything about that. I won't guess.",
                vec![],
            );
        }
        let read = tools.last().unwrap();
        let path = read.lines().next().unwrap().trim_start_matches("path: ");
        let fact = read
            .lines()
            .filter_map(|l| l.split_once(" | ").map(|(_, t)| t))
            .find(|t| t.contains("IU") || t.contains("diabetes") || t.contains("cousin"))
            .unwrap_or("")
            .to_owned();
        let sara_personal_readable = !tools[1].starts_with("ERROR");
        reply(
            &format!(
                "{fact} ([[{}|source]]). Also see [[vaults/health/imaginary-page|this]].\n{}",
                path.trim_end_matches(".md"),
                if sara_personal_readable {
                    ""
                } else {
                    "(personal pages closed)"
                }
            ),
            vec![],
        )
    })
}

fn runtime(p: MockProvider) -> AiRuntime {
    AiRuntime::new(ai_config(), Default::default()).with_provider("mock", Arc::new(p))
}

fn library() -> (Remote, Device) {
    let remote = Remote::new();
    let d = Device::clone_from(&remote, "laptop");
    page(
        &d,
        "vaults/health/labs/vitamin-d.md",
        "lab",
        "Vitamin D",
        "ویتامین D",
        "- Takes vitamin D 50,000 IU weekly (status:: confirmed) (src:: [[raw/2026/03/03/x|voice · 3 Mar]]) ^c-1",
    );
    page(
        &d,
        "vaults/life/people/sara.md",
        "person",
        "Sara",
        "سارا",
        "Sara is my cousin.",
    );
    page(
        &d,
        "vaults/stories/glass-city/characters/sara.md",
        "character",
        "Sara (Glass City)",
        "سارا",
        "In chapter 3 Sara is diagnosed with diabetes.",
    );
    (remote, d)
}

#[tokio::test(flavor = "multi_thread")]
async fn answers_cite_real_pages_and_do_not_invent() {
    let (_r, d) = library();
    let s = session(&d);
    let saw_image = Arc::new(Mutex::new(false));
    let rt = runtime(librarian(saw_image.clone()));
    let deltas = Mutex::new(String::new());
    let sink = |t: &str| deltas.lock().unwrap().push_str(t);

    let a = s
        .ask(
            &rt,
            &[],
            "How much vitamin D do I take?",
            None,
            &AskScope::All,
            None,
            &Cancel::default(),
            Some(&sink),
        )
        .await
        .unwrap();
    assert!(a.text.contains("50,000 IU"), "{}", a.text);
    assert!(a.text.contains("[[vaults/health/labs/vitamin-d|source]]"));
    assert!(
        !a.text.contains("imaginary-page"),
        "fabricated link is shown as text: {}",
        a.text
    );
    let real: Vec<_> = a.citations.iter().filter(|c| c.path.is_some()).collect();
    assert_eq!(real.len(), 1);
    assert_eq!(
        real[0].path.as_deref(),
        Some("vaults/health/labs/vitamin-d.md")
    );
    assert!(!deltas.lock().unwrap().is_empty(), "answer streamed");
    assert!(!a.needs_help);

    let a = s
        .ask(
            &rt,
            &[],
            "Where is my passport?",
            None,
            &AskScope::All,
            None,
            &Cancel::default(),
            None,
        )
        .await
        .unwrap();
    assert!(a.text.contains("doesn't say"), "{}", a.text);
    assert!(a.citations.is_empty());

    let png = vec![0x89, b'P', b'N', b'G'];
    s.ask(
        &rt,
        &[],
        "What does this say about Sara?",
        Some(("image/png".into(), png)),
        &AskScope::All,
        None,
        &Cancel::default(),
        None,
    )
    .await
    .unwrap();
    assert!(*saw_image.lock().unwrap(), "the photo reaches the model");

    let a = s
        .ask(
            &rt,
            &[],
            "Some days I just want to die. Is Sara around?",
            None,
            &AskScope::All,
            None,
            &Cancel::default(),
            None,
        )
        .await
        .unwrap();
    assert!(a.needs_help);
}

/// Story mode reads only the story; personal pages are closed to it and vice versa for claims.
#[tokio::test(flavor = "multi_thread")]
async fn story_scope_is_isolated() {
    let (_r, d) = library();
    let s = session(&d);
    let rt = runtime(librarian(Arc::default()));
    let a = s
        .ask(
            &rt,
            &[],
            "Does Sara have diabetes?",
            None,
            &AskScope::Story("glass-city".into()),
            None,
            &Cancel::default(),
            None,
        )
        .await
        .unwrap();
    assert!(a.text.contains("diagnosed with diabetes"), "{}", a.text);
    assert!(a.text.contains("(personal pages closed)"), "{}", a.text);
    assert!(a.citations.iter().all(|c| {
        c.path
            .as_deref()
            .is_none_or(|p| p.starts_with("vaults/stories/glass-city/"))
    }));

    let a = s
        .ask(
            &rt,
            &[],
            "Does Sara have diabetes?",
            None,
            &AskScope::Vault("health".into()),
            None,
            &Cancel::default(),
            None,
        )
        .await
        .unwrap();
    assert!(
        !a.text.contains("diagnosed"),
        "fiction never answers a health question: {}",
        a.text
    );

    let path = s
        .save_draft("glass-city", "Chapter 4 opening", "The glass hummed.")
        .unwrap();
    assert!(path.starts_with("vaults/stories/glass-city/drafts/"));
    assert!(d.read(&path).contains("The glass hummed."));
    let again = s
        .save_draft("glass-city", "Chapter 4 opening", "Second take.")
        .unwrap();
    assert_ne!(path, again, "drafts never overwrite each other");
}

/// "Save to wiki" files the answer through the normal pipeline.
#[tokio::test(flavor = "multi_thread")]
async fn saved_answers_are_filed() {
    let (_r, d) = library();
    let s = session(&d);
    let item = s
        .save_answer(
            "Who is Sara?",
            "Sara is your cousin ([[vaults/life/people/sara|Sara]]).",
            &AskScope::All,
        )
        .unwrap();
    assert_eq!(item.meta.kind, RawKind::ChatAnswer);
    let rt = runtime(archivist());
    let reports = s.run_jobs(&rt, true, &Cancel::default()).await.unwrap();
    assert!(!reports.is_empty());
    eprintln!("{reports:?}");
    assert_eq!(
        raw::read(&d.lib, &item.path).unwrap().meta.status,
        RawStatus::Ingested
    );
}
