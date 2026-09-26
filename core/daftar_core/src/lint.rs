//! Lint (§4.6): deterministic checks in code, judgement checks by the `lint` model whose quotes are
//! verified against the pages before anything is surfaced. Findings go to Review; only trivial,
//! deterministic fixes (index regeneration) are applied without approval.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use jiff::Zoned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::changeset::{self, Changeset, CommitInfo};
use crate::ledger::{LedgerEntry, OpType, Usage};
use crate::library::{Library, LocalDevice};
use crate::normalize::normalize;
use crate::ops::OpError;
use crate::providers::{ChatRequest, Message, Role};
use crate::queue::{JobState, Queue};
use crate::raw::{self, RawStatus};
use crate::review::{self, ReviewItem, ReviewKind};
use crate::runtime::AiRuntime;
use crate::{Result, pages, prompts, validate, wiki};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    BrokenLink,
    Orphan,
    BadFrontmatter,
    DuplicateSlug,
    DuplicateAlias,
    Oversized,
    StuckCapture,
    Contradiction,
    Stale,
    MissingPage,
    Merge,
    Split,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quote {
    pub path: String,
    pub text: String,
    /// 1-based line where the quote was found (filled by verification).
    #[serde(default)]
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    pub kind: FindingKind,
    pub summary: String,
    pub paths: Vec<String>,
    #[serde(default)]
    pub quotes: Vec<Quote>,
    #[serde(default)]
    pub fix: Option<String>,
}

impl Finding {
    /// Stable identity so the same problem is not queued twice.
    pub fn key(&self) -> String {
        let mut paths = self.paths.clone();
        paths.sort();
        format!(
            "{:?}|{}|{}",
            self.kind,
            paths.join(","),
            normalize(&self.summary)
        )
    }
}

/// Pages that are naturally not linked from anywhere.
fn orphan_exempt(p: &wiki::Page) -> bool {
    matches!(
        p.meta.kind.as_str(),
        "journal-day" | "review" | "profile" | "answer" | "draft" | "chapter" | "story" | "summary"
    ) || p.meta.status == "archived"
}

/// Code-only checks (§4.6).
pub fn deterministic(lib: &Library, queue: &Queue, now: &Zoned) -> Result<Vec<Finding>> {
    let mut out = Vec::new();
    let paths = pages::page_paths(lib)?;
    let resolver = pages::resolver(lib)?;
    let mut incoming: HashMap<String, usize> = HashMap::new();
    let mut loaded = Vec::new();
    for path in paths.iter().filter(|p| !pages::is_generated_index(p)) {
        let text = std::fs::read_to_string(lib.path(path)).unwrap_or_default();
        let n = text.lines().count();
        if n > validate::MAX_PAGE_LINES {
            out.push(Finding {
                kind: FindingKind::Oversized,
                summary: format!("{path} has {n} lines; consider splitting it."),
                paths: vec![path.clone()],
                quotes: vec![],
                fix: Some("Split it into pages by topic and link them.".into()),
            });
        }
        let page = match wiki::parse(path, &text) {
            Ok(p) => p,
            Err(e) => {
                out.push(Finding {
                    kind: FindingKind::BadFrontmatter,
                    summary: format!("The properties of {path} can't be read ({e})."),
                    paths: vec![path.clone()],
                    quotes: vec![],
                    fix: Some("Fix the lines between --- at the top.".into()),
                });
                continue;
            }
        };
        let mut problems = Vec::new();
        if !wiki::PAGE_TYPES.contains(&page.meta.kind.as_str()) {
            problems.push(format!("unknown type '{}'", page.meta.kind));
        }
        if page.meta.title.en.trim().is_empty() || page.meta.title.fa.trim().is_empty() {
            problems.push("the title needs both English and Persian".to_owned());
        }
        if !problems.is_empty() {
            out.push(Finding {
                kind: FindingKind::BadFrontmatter,
                summary: format!("{path}: {}.", problems.join("; ")),
                paths: vec![path.clone()],
                quotes: vec![],
                fix: None,
            });
        }
        for (i, line) in page.body.lines().enumerate() {
            for l in wiki::links(line) {
                if l.target.is_empty() || l.target == "talk-to-someone" {
                    continue;
                }
                match resolver.resolve(&l.target) {
                    Some(t) => {
                        if t != *path {
                            *incoming.entry(t).or_default() += 1;
                        }
                    }
                    None => out.push(Finding {
                        kind: FindingKind::BrokenLink,
                        summary: format!("[[{}]] on {path} does not point to a page.", l.target),
                        paths: vec![path.clone()],
                        quotes: vec![Quote {
                            path: path.clone(),
                            text: line.trim().to_owned(),
                            line: i + 1,
                        }],
                        fix: Some(format!(
                            "Create a page for {} or link an existing one.",
                            l.target
                        )),
                    }),
                }
            }
        }
        loaded.push(page);
    }
    for p in &loaded {
        if !orphan_exempt(p) && incoming.get(&p.path).copied().unwrap_or(0) == 0 {
            out.push(Finding {
                kind: FindingKind::Orphan,
                summary: format!("Nothing links to {}.", p.path),
                paths: vec![p.path.clone()],
                quotes: vec![],
                fix: Some("Link it from a related page, or archive it.".into()),
            });
        }
    }
    // Duplicate slugs and aliases among personal pages (stories keep their own namespace).
    let personal: Vec<&wiki::Page> = loaded
        .iter()
        .filter(|p| pages::vault_of(&p.path) != Some("stories"))
        .collect();
    let mut by_slug: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut by_alias: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for p in &personal {
        by_slug
            .entry(p.slug().to_lowercase())
            .or_default()
            .push(p.path.clone());
        for a in &p.meta.aliases {
            let n = normalize(a);
            if !n.is_empty() {
                by_alias.entry(n).or_default().insert(p.path.clone());
            }
        }
    }
    for (slug, ps) in by_slug.into_iter().filter(|(_, v)| v.len() > 1) {
        out.push(Finding {
            kind: FindingKind::DuplicateSlug,
            summary: format!(
                "{} pages are called '{slug}'; short links to them are ambiguous.",
                ps.len()
            ),
            paths: ps,
            quotes: vec![],
            fix: Some("Merge them or rename one.".into()),
        });
    }
    for (alias, ps) in by_alias.into_iter().filter(|(_, v)| v.len() > 1) {
        out.push(Finding {
            kind: FindingKind::DuplicateAlias,
            summary: format!("The alias '{alias}' is on {} pages.", ps.len()),
            paths: ps.into_iter().collect(),
            quotes: vec![],
            fix: Some("Keep the alias on the page it belongs to.".into()),
        });
    }
    // Captures stuck in `pending` for more than two days with nothing queued for them.
    let cutoff = now.timestamp().as_millisecond() - 2 * 24 * 3600 * 1000;
    for rel in pages::raw_paths(lib)? {
        let Ok(item) = raw::read(lib, &rel) else {
            continue;
        };
        if item.meta.status != RawStatus::Pending || item.id().timestamp_ms() as i64 > cutoff {
            continue;
        }
        let jobs = queue.jobs_for_raw(item.id())?;
        if jobs
            .iter()
            .any(|j| matches!(j.state, JobState::Queued | JobState::Running))
        {
            continue;
        }
        out.push(Finding {
            kind: FindingKind::StuckCapture,
            summary: format!(
                "A capture from {} was never filed.",
                &item.meta.captured_at[..10]
            ),
            paths: vec![rel],
            quotes: vec![],
            fix: Some("Retry filing it, or exclude it.".into()),
        });
    }
    Ok(out)
}

/// Keeps only findings whose every quote is really on the named page (whitespace-insensitive), and
/// never lets a story contradict the user's own pages.
pub fn verify(lib: &Library, findings: Vec<Finding>) -> Vec<Finding> {
    let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut cache: HashMap<String, Option<String>> = HashMap::new();
    findings
        .into_iter()
        .filter_map(|mut f| {
            if f.quotes.is_empty() {
                return None;
            }
            for q in &mut f.quotes {
                let text = cache
                    .entry(q.path.clone())
                    .or_insert_with(|| {
                        (q.path.starts_with("vaults/") && !q.path.contains(".."))
                            .then(|| std::fs::read_to_string(lib.path(&q.path)).ok())
                            .flatten()
                    })
                    .clone()?;
                let want = squash(&q.text);
                if want.chars().count() < 8 {
                    return None;
                }
                q.line = text
                    .lines()
                    .position(|l| squash(l).contains(&want))
                    .map(|i| i + 1)?;
            }
            let stories = f
                .quotes
                .iter()
                .filter(|q| q.path.starts_with("vaults/stories/"))
                .count();
            if f.kind == FindingKind::Contradiction && stories > 0 && stories < f.quotes.len() {
                return None;
            }
            let mut paths: Vec<String> = f.quotes.iter().map(|q| q.path.clone()).collect();
            paths.sort();
            paths.dedup();
            f.paths = paths;
            Some(f)
        })
        .collect()
}

/// Judgement checks by the `lint` model over `paths` (§4.6). Returns verified findings.
pub async fn judged(
    lib: &Library,
    rt: &AiRuntime,
    paths: &[String],
    now: &Zoned,
) -> std::result::Result<(Vec<Finding>, Usage, String), OpError> {
    let (p, rc) = rt.for_role(Role::Lint)?;
    let config = lib.config()?;
    let schema = prompts::schema(lib);
    let mut body = String::new();
    for path in paths {
        if let Ok(t) = std::fs::read_to_string(lib.path(path)) {
            body.push_str(&format!("\n<page path=\"{path}\">\n{t}\n</page>\n"));
        }
    }
    let system = prompts::render(
        prompts::LINT,
        &[
            ("today", &now.strftime("%Y-%m-%d").to_string()),
            (
                "timezone",
                now.time_zone().iana_name().unwrap_or("local time"),
            ),
            ("languages", "Persian (fa) and English (en)"),
            ("vaults", &config.vaults_for_prompt()),
            ("schema", &schema),
            ("pages", &paths.join("\n")),
        ],
    );
    let req = ChatRequest {
        model: rc.model.clone(),
        system,
        messages: vec![Message::user(format!("LINT task. The pages:\n{body}"))],
        tools: vec![],
        max_tokens: 4000,
        temperature: Some(0.0),
        json: true,
        params: rc.params.clone(),
    };
    let resp = p.chat(&req, None).await?;
    let v = crate::ops::extract_json(&resp.text).unwrap_or(json!({"findings": []}));
    let raw: Vec<Value> = v["findings"].as_array().cloned().unwrap_or_default();
    let findings: Vec<Finding> = raw
        .into_iter()
        .filter_map(|f| {
            let kind = match f["kind"].as_str()? {
                "contradiction" => FindingKind::Contradiction,
                "stale" => FindingKind::Stale,
                "missing_page" => FindingKind::MissingPage,
                "merge" => FindingKind::Merge,
                "split" => FindingKind::Split,
                _ => return None,
            };
            Some(Finding {
                kind,
                summary: f["summary"].as_str()?.to_owned(),
                paths: vec![],
                quotes: serde_json::from_value(f["quotes"].clone()).ok()?,
                fix: f["fix"].as_str().map(str::to_owned),
            })
        })
        .collect();
    let mut usage = resp.usage;
    usage.cost_usd = rt.config.cost(&rc.model, &usage);
    Ok((
        verify(lib, findings),
        usage,
        format!("{}/{}", rc.provider, rc.model),
    ))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LintReport {
    pub op_id: Option<String>,
    pub findings: Vec<Finding>,
    /// New Review cards (findings already waiting in Review are not added again).
    pub new_cards: usize,
    pub indexes_regenerated: usize,
}

/// One lint pass: deterministic checks, optional model checks over `judge` pages, findings to
/// Review, indexes regenerated. One commit and one ledger entry when anything changed.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    lib: &Library,
    dev: &LocalDevice,
    checked: Vec<Finding>,
    rt: Option<&AiRuntime>,
    judge: &[String],
    now: &Zoned,
    commit_lock: &std::sync::Mutex<()>,
) -> std::result::Result<LintReport, OpError> {
    // `checked` are the deterministic findings (`deterministic`), computed by the caller so no
    // queue handle is held across the model call.
    let mut findings = checked;
    let mut usage = Usage::default();
    let mut models = vec![];
    if let (Some(rt), false) = (rt, judge.is_empty()) {
        let (f, u, m) = judged(lib, rt, judge, now).await?;
        findings.extend(f);
        usage = u;
        models = vec![m, prompts::version(prompts::LINT).to_owned()];
    }
    let _g = commit_lock.lock().unwrap_or_else(|p| p.into_inner());
    let open: BTreeSet<String> = review::list(lib)?
        .into_iter()
        .filter(|r| r.kind == ReviewKind::Lint)
        .filter_map(|r| r.payload["key"].as_str().map(str::to_owned))
        .collect();
    let op_id = crate::ids::new_id().to_string();
    let mut cs = Changeset::default();
    for f in &findings {
        let key = f.key();
        if open.contains(&key) {
            continue;
        }
        let mut payload = serde_json::to_value(f).map_err(crate::Error::from)?;
        payload["key"] = Value::String(key);
        cs.review_items.push(ReviewItem::new(
            ReviewKind::Lint,
            &dev.id,
            now,
            Some(op_id.clone()),
            payload,
        ));
    }
    let vaults: Vec<String> = lib
        .config()?
        .active_vaults()
        .map(|v| v.id.clone())
        .collect();
    let regenerated = pages::regenerate_indexes(lib, &vaults)?;
    let report = LintReport {
        op_id: None,
        new_cards: cs.review_items.len(),
        indexes_regenerated: regenerated.len(),
        findings,
    };
    if cs.review_items.is_empty() && regenerated.is_empty() {
        return Ok(report);
    }
    // The regenerated indexes are already on disk; include them in this commit.
    for rel in &regenerated {
        let text = std::fs::read_to_string(lib.path(rel)).map_err(crate::Error::from)?;
        cs.files.insert(rel.clone(), Some(text));
    }
    let summary = format!(
        "Checked the wiki: {} finding{} to review{}",
        report.new_cards,
        if report.new_cards == 1 { "" } else { "s" },
        if regenerated.is_empty() {
            String::new()
        } else {
            format!(", {} indexes rebuilt", regenerated.len())
        }
    );
    let entry = LedgerEntry {
        op_id: op_id.clone(),
        op_type: OpType::Lint,
        sources: vec![],
        router: None,
        models,
        pages_created: vec![],
        pages_updated: vec![],
        claims_added: vec![],
        review_items: vec![],
        usage,
        started_at: crate::time::rfc3339(now),
        finished_at: crate::time::rfc3339(now),
        device: dev.id.clone(),
        summary: summary.clone(),
        note: None,
        forced_vault: None,
        replayed_from: None,
        reverts: None,
        rejected_claims: vec![],
    };
    changeset::commit(
        lib,
        dev,
        now,
        &cs,
        entry,
        CommitInfo {
            subject: format!("lint: {} findings", report.new_cards),
            source_path: None,
            log_title: summary,
        },
    )?;
    Ok(LintReport {
        op_id: Some(op_id),
        ..report
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::lib_in;

    fn page(lib: &Library, path: &str, kind: &str, aliases: &str, body: &str) {
        crate::fsutil::atomic_write(
            &lib.path(path),
            format!("---\ntype: {kind}\ntitle: {{ en: \"T\", fa: \"ت\" }}\naliases: [{aliases}]\nsummary: \"s\"\n---\n\n{body}\n").as_bytes(),
        )
        .unwrap();
    }

    #[test]
    fn deterministic_checks() {
        let (_d, lib) = lib_in();
        let q = Queue::in_memory().unwrap();
        page(
            &lib,
            "vaults/life/people/sara.md",
            "person",
            "\"sara\"",
            "Cousin. Knows [[people/ali]] and [[nobody-here]].",
        );
        page(
            &lib,
            "vaults/life/people/ali.md",
            "person",
            "",
            "Friend of [[sara]].",
        );
        page(
            &lib,
            "vaults/work/topics/sara.md",
            "topic",
            "\"sara\"",
            "Project codename.",
        );
        page(
            &lib,
            "vaults/health/profile.md",
            "profile",
            "",
            "Nothing links here, which is fine.",
        );
        page(
            &lib,
            "vaults/mind/patterns/lonely.md",
            "pattern",
            "",
            "Nobody links here.",
        );
        let now = crate::testutil::zoned("2026-09-23T20:00:00+03:30[Asia/Tehran]");
        let f = deterministic(&lib, &q, &now).unwrap();
        let kinds = |k: FindingKind| f.iter().filter(|x| x.kind == k).collect::<Vec<_>>();
        assert_eq!(
            kinds(FindingKind::BrokenLink).len(),
            2,
            "[[nobody-here]] and the now-ambiguous [[sara]]: {f:#?}"
        );
        assert_eq!(kinds(FindingKind::BrokenLink)[0].quotes[0].line, 1);
        assert!(
            kinds(FindingKind::Orphan)
                .iter()
                .any(|x| x.paths[0] == "vaults/mind/patterns/lonely.md")
        );
        assert!(
            !kinds(FindingKind::Orphan)
                .iter()
                .any(|x| x.paths[0] == "vaults/health/profile.md")
        );
        assert_eq!(kinds(FindingKind::DuplicateSlug).len(), 1);
        assert_eq!(kinds(FindingKind::DuplicateAlias).len(), 1);
    }

    #[test]
    fn only_verifiable_quotes_survive() {
        let (_d, lib) = lib_in();
        page(
            &lib,
            "vaults/health/profile.md",
            "profile",
            "",
            "- Takes vitamin D 50,000 IU weekly (status:: confirmed) ^c-1",
        );
        page(
            &lib,
            "vaults/health/labs/vit-d.md",
            "lab",
            "",
            "- Stopped vitamin D in March (status:: confirmed) ^c-2",
        );
        page(
            &lib,
            "vaults/stories/x/characters/sara.md",
            "character",
            "",
            "Sara takes vitamin D daily.",
        );
        let f = |quotes: Vec<(&str, &str)>| Finding {
            kind: FindingKind::Contradiction,
            summary: "x".into(),
            paths: vec![],
            quotes: quotes
                .into_iter()
                .map(|(p, t)| Quote {
                    path: p.into(),
                    text: t.into(),
                    line: 0,
                })
                .collect(),
            fix: None,
        };
        let kept = verify(
            &lib,
            vec![
                f(vec![
                    (
                        "vaults/health/profile.md",
                        "Takes vitamin D  50,000 IU weekly",
                    ),
                    ("vaults/health/labs/vit-d.md", "Stopped vitamin D in March"),
                ]),
                f(vec![(
                    "vaults/health/profile.md",
                    "Takes vitamin D 60,000 IU weekly",
                )]),
                f(vec![
                    (
                        "vaults/health/profile.md",
                        "Takes vitamin D 50,000 IU weekly",
                    ),
                    (
                        "vaults/stories/x/characters/sara.md",
                        "Sara takes vitamin D daily",
                    ),
                ]),
                f(vec![]),
            ],
        );
        assert_eq!(kept.len(), 1, "{kept:#?}");
        assert_eq!(kept[0].quotes[0].line, 8);
        assert_eq!(kept[0].paths.len(), 2);
    }
}
