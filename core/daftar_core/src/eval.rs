//! Prompt regression harness (§15, `daftar eval`). A case sets up a small library, runs one op with
//! the configured provider, and checks structural expectations: vaults and pages touched, claims
//! proposed, citations present and real, isolation respected, unknowns admitted.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::agent::Cancel;
use crate::ask::AskScope;
use crate::ledger::{self, OpType};
use crate::library::Library;
use crate::runtime::AiRuntime;
use crate::session::Session;
use crate::{Result, pages, review, wiki};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageFixture {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CaptureFixture {
    pub text: String,
    /// Zoned time, e.g. `2026-09-23T08:12:00+03:30[Asia/Tehran]`.
    pub at: String,
    #[serde(default)]
    pub vault: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Expect {
    /// Every one of these vaults is written to.
    #[serde(default)]
    pub vaults: Vec<String>,
    /// Nothing is written under these prefixes.
    #[serde(default)]
    pub never_write: Vec<String>,
    #[serde(default)]
    pub pages_touched_min: usize,
    #[serde(default)]
    pub claims_proposed_min: usize,
    /// Some page written links to each of these (path without `.md`).
    #[serde(default)]
    pub links_to: Vec<String>,
    /// A journal day section cites the capture.
    #[serde(default)]
    pub journal_section: bool,
    /// Ask: the answer cites each of these pages or captures.
    #[serde(default)]
    pub cites: Vec<String>,
    /// Ask: the answer has at least one real citation.
    #[serde(default)]
    pub cited: bool,
    /// Ask: the answer says the wiki does not know (no personal facts invented).
    #[serde(default)]
    pub admits_unknown: bool,
    /// Ask: none of these strings appear in the answer (e.g. a fabricated fact).
    #[serde(default)]
    pub never_says: Vec<String>,
    /// Lint: finding kinds that must be reported.
    #[serde(default)]
    pub finding_kinds: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Case {
    pub name: String,
    /// `ingest`, `query`, `story`, `lint`, `reflect_daily`.
    pub op: String,
    #[serde(default)]
    pub pages: Vec<PageFixture>,
    /// Filed first (setup) for query/lint/reflect cases; the last one is the case input for ingest.
    #[serde(default)]
    pub captures: Vec<CaptureFixture>,
    #[serde(default)]
    pub question: Option<String>,
    #[serde(default)]
    pub story: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    pub expect: Expect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseResult {
    pub name: String,
    pub passed: bool,
    pub failures: Vec<String>,
    pub notes: Vec<String>,
}

const UNKNOWN_MARKERS: &[&str] = &[
    "doesn't say",
    "does not say",
    "doesn't mention",
    "does not mention",
    "no record",
    "not in your",
    "don't know",
    "do not know",
    "couldn't find",
    "could not find",
    "nothing about",
    "اطلاعاتی",
    "چیزی درباره",
    "پیدا نکردم",
    "نمی‌دانم",
    "ثبت نشده",
    "اشاره‌ای نشده",
];

fn written_since(lib: &Library, before: &BTreeSet<String>) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for e in ledger::all(lib)? {
        if before.contains(&e.op_id) {
            continue;
        }
        out.extend(e.pages_created.iter().cloned());
        out.extend(e.pages_updated.iter().cloned());
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Runs one case in a fresh library under `dir`.
pub async fn run_case(case: &Case, rt: &AiRuntime, dir: &std::path::Path) -> Result<CaseResult> {
    let lib = crate::sync::init_local(dir, "main")?;
    lib.set_device("eval", "eval", &jiff::Zoned::now())?;
    let mut c = lib.config()?;
    c.ai = rt.config.clone();
    lib.save_config(&c)?;
    for p in &case.pages {
        crate::fsutil::atomic_write(&lib.path(&p.path), p.content.as_bytes())?;
    }
    let s = Session::open(lib.root())?;
    let cancel = Cancel::default();
    let mut failures = Vec::new();
    let mut notes = Vec::new();
    let e = &case.expect;

    // Setup captures (all but the last for ingest cases) are filed first.
    let setup = match case.op.as_str() {
        "ingest" => case.captures.len().saturating_sub(1),
        _ => case.captures.len(),
    };
    for cap in &case.captures[..setup] {
        let at: jiff::Zoned = cap.at.parse()?;
        s.capture_text(&cap.text, cap.vault.clone(), &at)?;
    }
    if setup > 0 {
        for r in s.run_jobs(rt, true, &cancel).await? {
            if r.state != crate::queue::JobState::Done {
                failures.push(format!(
                    "setup {:?} failed: {}",
                    r.kind,
                    r.message.unwrap_or_default()
                ));
            }
        }
    }
    let before: BTreeSet<String> = ledger::all(&lib)?.into_iter().map(|e| e.op_id).collect();

    match case.op.as_str() {
        "ingest" => {
            let Some(cap) = case.captures.last() else {
                return Err(crate::Error::invalid("an ingest case needs a capture"));
            };
            let at: jiff::Zoned = cap.at.parse()?;
            let item = s.capture_text(&cap.text, cap.vault.clone(), &at)?;
            for r in s.run_jobs(rt, true, &cancel).await? {
                if r.state != crate::queue::JobState::Done {
                    failures.push(format!(
                        "{:?} failed: {}",
                        r.kind,
                        r.message.unwrap_or_default()
                    ));
                }
            }
            let written = written_since(&lib, &before)?;
            notes.push(format!("wrote {}", written.join(", ")));
            for v in &e.vaults {
                if !written
                    .iter()
                    .any(|p| pages::vault_of(p) == Some(v.as_str()))
                {
                    failures.push(format!("nothing written to the {v} vault"));
                }
            }
            for n in &e.never_write {
                if let Some(p) = written.iter().find(|p| p.starts_with(n.as_str())) {
                    failures.push(format!("wrote {p}, under {n}"));
                }
            }
            if written.len() < e.pages_touched_min {
                failures.push(format!(
                    "{} pages touched, expected at least {}",
                    written.len(),
                    e.pages_touched_min
                ));
            }
            let claims = review::list(&lib)?
                .into_iter()
                .filter(|r| r.kind == review::ReviewKind::Claim)
                .count();
            if claims < e.claims_proposed_min {
                failures.push(format!(
                    "{claims} claims proposed, expected at least {}",
                    e.claims_proposed_min
                ));
            }
            let texts: Vec<String> = written
                .iter()
                .filter_map(|p| std::fs::read_to_string(lib.path(p)).ok())
                .collect();
            for target in &e.links_to {
                let hit = texts.iter().flat_map(|t| wiki::links(t)).any(|l| {
                    l.target.trim_end_matches(".md") == target
                        || target.ends_with(&format!("/{}", l.target))
                });
                if !hit {
                    failures.push(format!("no link to {target}"));
                }
            }
            if e.journal_section {
                let raw_no_ext = item.path.trim_end_matches(".md");
                let journal = written
                    .iter()
                    .filter(|p| p.starts_with("vaults/life/journal/"))
                    .filter_map(|p| std::fs::read_to_string(lib.path(p)).ok())
                    .any(|t| t.contains(raw_no_ext));
                if !journal {
                    failures.push("no journal section citing the capture".into());
                }
            }
        }
        "query" | "story" => {
            let q = case.question.clone().unwrap_or_default();
            let scope = match (&case.op[..], &case.story) {
                ("story", Some(st)) => AskScope::Story(st.clone()),
                _ => AskScope::All,
            };
            let a = s
                .ask(rt, &[], &q, None, &scope, None, &cancel, None)
                .await
                .map_err(|e| crate::Error::Other(e.to_string()))?;
            notes.push(a.text.chars().take(300).collect());
            let real: Vec<String> = a.citations.iter().filter_map(|c| c.path.clone()).collect();
            let dead = a.citations.iter().filter(|c| c.path.is_none()).count();
            if dead > 0 {
                notes.push(format!(
                    "{dead} citation(s) to things that do not exist were removed"
                ));
            }
            for c in &e.cites {
                if !real.iter().any(|p| p.trim_end_matches(".md") == c) {
                    failures.push(format!("does not cite {c}"));
                }
            }
            if e.cited && real.is_empty() {
                failures.push("no real citation".into());
            }
            let lower = a.text.to_lowercase();
            if e.admits_unknown && !UNKNOWN_MARKERS.iter().any(|m| lower.contains(m)) {
                failures.push("does not say the wiki lacks the answer".into());
            }
            for n in &e.never_says {
                if lower.contains(&n.to_lowercase()) {
                    failures.push(format!("says \"{n}\""));
                }
            }
            if let AskScope::Story(st) = &scope
                && let Some(p) = real
                    .iter()
                    .find(|p| !p.starts_with(&format!("vaults/stories/{st}/")))
            {
                failures.push(format!("story answer cites {p}"));
            }
        }
        "lint" => {
            let judge: Vec<String> = case.pages.iter().map(|p| p.path.clone()).collect();
            let checked = crate::lint::deterministic(
                &lib,
                &crate::queue::Queue::in_memory()?,
                &jiff::Zoned::now(),
            )?;
            let lock = std::sync::Mutex::new(());
            let r = crate::lint::run(
                &lib,
                s.device(),
                checked,
                Some(rt),
                &judge,
                &jiff::Zoned::now(),
                &lock,
            )
            .await
            .map_err(|e| crate::Error::Other(e.to_string()))?;
            let kinds: BTreeSet<String> = r
                .findings
                .iter()
                .map(|f| {
                    serde_json::to_value(f.kind)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_default()
                })
                .collect();
            notes.push(format!("found {kinds:?}"));
            for k in &e.finding_kinds {
                if !kinds.contains(k) {
                    failures.push(format!("did not report {k}"));
                }
            }
        }
        "reflect_daily" => {
            let date = case.date.clone().unwrap_or_default();
            let lock = std::sync::Mutex::new(());
            let out = crate::reflect::daily(
                &lib,
                s.device(),
                rt,
                date.parse()?,
                true,
                &jiff::Zoned::now(),
                &lock,
            )
            .await
            .map_err(|e| crate::Error::Other(e.to_string()))?;
            match out.page {
                Some(p) => {
                    let t = std::fs::read_to_string(lib.path(&p))?;
                    let summary = t.split("## Day summary").nth(1).unwrap_or_default();
                    if summary.trim().is_empty() {
                        failures.push("no day summary".into());
                    } else if !summary.contains("[[raw/") {
                        failures.push("the summary cites no capture".into());
                    }
                    if ledger::all(&lib)?
                        .iter()
                        .filter(|e| e.op_type == OpType::Reflect)
                        .count()
                        != 1
                    {
                        failures.push("expected exactly one reflect op".into());
                    }
                }
                None => failures.push("no journal page for that day".into()),
            }
        }
        other => return Err(crate::Error::invalid(format!("unknown eval op {other}"))),
    }
    Ok(CaseResult {
        name: case.name.clone(),
        passed: failures.is_empty(),
        failures,
        notes,
    })
}

/// Loads every `*.json` case under `dir` (sorted by file name).
pub fn load_cases(dir: &std::path::Path) -> Result<Vec<Case>> {
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    files
        .iter()
        .map(|p| Ok(serde_json::from_slice(&std::fs::read(p)?)?))
        .collect()
}
