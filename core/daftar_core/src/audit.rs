//! Audit, undo and correction (§7): the Activity list, readable per-page diffs of an op, and undo
//! as `git revert` of the op commit, falling back to a compensating op when later commits make the
//! revert conflict.
//!
//! A revert only takes the op's changes to wiki pages and Review items. The ledger and the log are
//! append-only (the undo gets its own entries), vault indexes are regenerated, and the capture's
//! status is set explicitly: excluded after an undo, pending before a move or re-run, ingested again
//! when an undo is itself undone.

use std::collections::BTreeSet;

use git2::{DiffOptions, Oid, Repository};
use jiff::Zoned;
use serde::{Deserialize, Serialize};

use crate::agent::{self, AgentSpec, Cancel};
use crate::changeset::{self, Changeset, CommitInfo};
use crate::ledger::{self, LedgerEntry, OpType};
use crate::library::{Library, LocalDevice};
use crate::ops::OpError;
use crate::providers::{Message, Role};
use crate::raw::{self, RawStatus};
use crate::runtime::AiRuntime;
use crate::tools::{self, OpContext, Scope};
use crate::{Error, Result, pages, prompts, review, validate};

// ─────────────────────────── activity ───────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpSummary {
    pub op_id: String,
    pub op_type: OpType,
    pub summary: String,
    pub started_at: String,
    pub device: String,
    pub vaults: Vec<String>,
    pub pages_created: usize,
    pub pages_updated: usize,
    pub claims_added: usize,
    pub sources: Vec<String>,
    /// The op this one undoes, for revert and compensate ops.
    pub reverts: Option<String>,
    pub replayed_from: Option<String>,
    /// Undone (and not re-done).
    pub reverted: bool,
    pub note: Option<String>,
    pub forced_vault: Option<String>,
    pub models: Vec<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<f64>,
    /// Router decision as recorded: targets with reasons and confidences.
    pub router: Option<serde_json::Value>,
}

fn vaults_of(e: &LedgerEntry) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in e.pages_created.iter().chain(&e.pages_updated) {
        if let Some(v) = pages::vault_of(p)
            && !out.iter().any(|x| x == v)
        {
            out.push(v.to_owned());
        }
    }
    out
}

fn summarize(e: &LedgerEntry, reverted: &std::collections::HashSet<String>) -> OpSummary {
    OpSummary {
        op_id: e.op_id.clone(),
        op_type: e.op_type,
        summary: e.summary.clone(),
        started_at: e.started_at.clone(),
        device: e.device.clone(),
        vaults: vaults_of(e),
        pages_created: e.pages_created.len(),
        pages_updated: e.pages_updated.len(),
        claims_added: e.claims_added.len(),
        sources: e.sources.clone(),
        reverts: e.reverts.clone(),
        replayed_from: e.replayed_from.clone(),
        reverted: reverted.contains(&e.op_id),
        note: e.note.clone(),
        forced_vault: e.forced_vault.clone(),
        models: e.models.clone(),
        input_tokens: e.usage.input_tokens,
        output_tokens: e.usage.output_tokens,
        cost_usd: e.usage.cost_usd,
        router: e.router.clone(),
    }
}

/// Ops newest first, optionally only those older than `before` (an op id), for paging.
pub fn activity(lib: &Library, limit: usize, before: Option<&str>) -> Result<Vec<OpSummary>> {
    let all = ledger::all(lib)?;
    let reverted = ledger::reverted_ops(&all);
    Ok(all
        .iter()
        .rev()
        .filter(|e| before.is_none_or(|b| e.op_id.as_str() < b))
        .take(limit)
        .map(|e| summarize(e, &reverted))
        .collect())
}

pub fn op(lib: &Library, op_id: &str) -> Result<OpSummary> {
    let all = ledger::all(lib)?;
    let reverted = ledger::reverted_ops(&all);
    all.iter()
        .find(|e| e.op_id == op_id)
        .map(|e| summarize(e, &reverted))
        .ok_or_else(|| Error::invalid("That operation is not in the ledger."))
}

// ─────────────────────────── diffs ───────────────────────────

/// The commit of an op, found through its `Op-Id` trailer (§5.5).
pub fn op_commit(repo: &Repository, op_id: &str) -> Result<Option<Oid>> {
    let mut walk = repo.revwalk()?;
    if walk.push_head().is_err() {
        return Ok(None);
    }
    for oid in walk {
        let oid = oid?;
        let c = repo.find_commit(oid)?;
        if ledger::trailer(c.message().unwrap_or_default(), "Op-Id") == Some(op_id) {
            return Ok(Some(oid));
        }
    }
    Ok(None)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineKind {
    Context,
    Added,
    Removed,
    /// Separator between hunks.
    Gap,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffLine {
    pub kind: LineKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChange {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub change: FileChange,
    pub lines: Vec<DiffLine>,
}

/// Wiki pages an op changed (not generated indexes), shown as a reader-friendly line diff.
pub fn op_diff(lib: &Library, op_id: &str) -> Result<Vec<FileDiff>> {
    let repo = Repository::open(lib.root())?;
    let oid = op_commit(&repo, op_id)?
        .ok_or_else(|| Error::invalid("This operation's commit is not on this device yet."))?;
    commit_page_diff(&repo, oid)
}

fn is_page(path: &str) -> bool {
    path.starts_with("vaults/") && path.ends_with(".md") && !pages::is_generated_index(path)
}

fn commit_page_diff(repo: &Repository, oid: Oid) -> Result<Vec<FileDiff>> {
    let c = repo.find_commit(oid)?;
    let new = c.tree()?;
    let old = c.parent(0).ok().map(|p| p.tree()).transpose()?;
    let mut opts = DiffOptions::new();
    opts.context_lines(2);
    let diff = repo.diff_tree_to_tree(old.as_ref(), Some(&new), Some(&mut opts))?;
    let mut out: Vec<FileDiff> = Vec::new();
    diff.print(git2::DiffFormat::Patch, |delta, hunk, line| {
        let path = delta
            .new_file()
            .path()
            .or(delta.old_file().path())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        if !is_page(&path) {
            return true;
        }
        if out.last().is_none_or(|f| f.path != path) {
            out.push(FileDiff {
                path: path.clone(),
                change: match delta.status() {
                    git2::Delta::Added => FileChange::Added,
                    git2::Delta::Deleted => FileChange::Deleted,
                    _ => FileChange::Modified,
                },
                lines: vec![],
            });
        }
        let f = out.last_mut().expect("pushed above");
        let kind = match line.origin() {
            '+' => LineKind::Added,
            '-' => LineKind::Removed,
            ' ' => LineKind::Context,
            'H' => {
                if hunk.is_some() && !f.lines.is_empty() {
                    f.lines.push(DiffLine {
                        kind: LineKind::Gap,
                        text: String::new(),
                    });
                }
                return true;
            }
            _ => return true,
        };
        let text = String::from_utf8_lossy(line.content())
            .trim_end_matches(['\n', '\r'])
            .to_owned();
        f.lines.push(DiffLine { kind, text });
        true
    })?;
    Ok(out)
}

/// Unified diff text of an op's page changes, as given to the compensate prompt.
fn diff_text(files: &[FileDiff]) -> String {
    let mut s = String::new();
    for f in files {
        s.push_str(&format!("--- {}\n", f.path));
        for l in &f.lines {
            let p = match l.kind {
                LineKind::Added => "+",
                LineKind::Removed => "-",
                LineKind::Context => " ",
                LineKind::Gap => "@@",
            };
            s.push_str(p);
            s.push_str(&l.text);
            s.push('\n');
        }
    }
    s
}

// ─────────────────────────── undo ───────────────────────────

/// What an undone capture becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AfterUndo {
    /// Plain undo / exclude: the capture stays in raw/ as `excluded` and can be re-included.
    Exclude,
    /// Move to vault / re-run with note: the capture goes back to `pending` for a new ingest.
    Refile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UndoOutcome {
    /// Reverted cleanly; the id of the new `revert-op`.
    Reverted(String),
    /// Later ops changed the same lines; a compensating op is needed.
    NeedsCompensation,
}

fn load(lib: &Library, op_id: &str) -> Result<LedgerEntry> {
    ledger::all(lib)?
        .into_iter()
        .find(|e| e.op_id == op_id)
        .ok_or_else(|| Error::invalid("That operation is not in the ledger."))
}

/// Raw status changes that go with undoing `target`.
fn status_after(
    lib: &Library,
    target: &LedgerEntry,
    after: AfterUndo,
    cs: &mut Changeset,
) -> Result<Vec<String>> {
    let all = ledger::all(lib)?;
    let (ids, status) = match (target.op_type, &target.reverts) {
        // Undoing an undo: the original op is live again, so its captures are filed again.
        (OpType::RevertOp | OpType::Compensate, Some(orig)) => {
            let sources = all
                .iter()
                .find(|e| &e.op_id == orig)
                .map(|e| e.sources.clone())
                .unwrap_or_default();
            (sources, RawStatus::Ingested)
        }
        _ => (
            target.sources.clone(),
            match after {
                AfterUndo::Exclude => RawStatus::Excluded,
                AfterUndo::Refile => RawStatus::Pending,
            },
        ),
    };
    let mut paths = Vec::new();
    for id in &ids {
        if let Ok(uid) = id.parse()
            && let Some(item) = raw::find(lib, uid)?
        {
            cs.raw_status.insert(item.path.clone(), status);
            paths.push(item.path);
        }
    }
    Ok(paths)
}

fn undo_entry(
    target: &LedgerEntry,
    op_type: OpType,
    dev: &LocalDevice,
    now: &Zoned,
    summary: String,
) -> LedgerEntry {
    LedgerEntry {
        op_id: crate::ids::new_id().to_string(),
        op_type,
        sources: target.sources.clone(),
        router: None,
        models: vec![],
        pages_created: vec![],
        pages_updated: vec![],
        claims_added: vec![],
        review_items: vec![],
        usage: Default::default(),
        started_at: crate::time::rfc3339(now),
        finished_at: crate::time::rfc3339(now),
        device: dev.id.clone(),
        summary,
        note: None,
        forced_vault: None,
        replayed_from: None,
        reverts: Some(target.op_id.clone()),
        rejected_claims: vec![],
    }
}

fn undo_summary(target: &LedgerEntry) -> String {
    if target.summary.is_empty() {
        format!("Undid {} {}", target.op_type.as_str(), target.op_id)
    } else {
        format!("Undid: {}", target.summary)
    }
}

/// Undo by `git revert` of the op commit. Must run with the commit lock held and after local
/// changes were committed (so files on disk match HEAD).
pub fn try_revert(
    lib: &Library,
    dev: &LocalDevice,
    now: &Zoned,
    op_id: &str,
    after: AfterUndo,
) -> Result<UndoOutcome> {
    let target = load(lib, op_id)?;
    let all = ledger::all(lib)?;
    if ledger::reverted_ops(&all).contains(op_id) {
        return Err(Error::invalid("This operation is already undone."));
    }
    let repo = Repository::open(lib.root())?;
    let oid = op_commit(&repo, op_id)?
        .ok_or_else(|| Error::invalid("This operation's commit is not on this device yet."))?;
    let commit = repo.find_commit(oid)?;
    let head = repo.head()?.peel_to_commit()?;
    let idx = repo.revert_commit(&commit, &head, 0, None)?;
    let relevant = |p: &str| is_page(p) || p.starts_with(review::DIR);

    if idx.has_conflicts() {
        for c in idx.conflicts()? {
            let c = c?;
            let path = c
                .our
                .as_ref()
                .or(c.their.as_ref())
                .or(c.ancestor.as_ref())
                .map(|e| String::from_utf8_lossy(&e.path).into_owned())
                .unwrap_or_default();
            if relevant(&path) {
                return Ok(UndoOutcome::NeedsCompensation);
            }
        }
    }

    // Paths the op commit touched, restricted to pages and Review items.
    let parent_tree = commit.parent(0).ok().map(|p| p.tree()).transpose()?;
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit.tree()?), None)?;
    let mut touched = BTreeSet::new();
    for d in diff.deltas() {
        for f in [d.old_file(), d.new_file()] {
            if let Some(p) = f.path().map(|p| p.to_string_lossy().replace('\\', "/"))
                && relevant(&p)
            {
                touched.insert(p);
            }
        }
    }

    let mut cs = Changeset::default();
    for p in &touched {
        let content = match idx.get_path(std::path::Path::new(p), 0) {
            Some(e) => Some(String::from_utf8_lossy(repo.find_blob(e.id)?.content()).into_owned()),
            None => None,
        };
        let current = std::fs::read_to_string(lib.path(p)).ok();
        if content != current {
            cs.files.insert(p.clone(), content);
        }
    }
    let source_paths = status_after(lib, &target, after, &mut cs)?;
    let entry = undo_entry(&target, OpType::RevertOp, dev, now, undo_summary(&target));
    let new_id = entry.op_id.clone();
    changeset::commit(
        lib,
        dev,
        now,
        &cs,
        entry,
        CommitInfo {
            subject: format!(
                "revert-op: undo {} {}",
                target.op_type.as_str(),
                target.op_id
            ),
            source_path: source_paths.first().cloned(),
            log_title: target.summary.clone(),
        },
    )?;
    Ok(UndoOutcome::Reverted(new_id))
}

/// Undo through the model (§7): removes only what came solely from the op's source, via the
/// normal tools and validator, and commits one `compensate` op.
#[allow(clippy::too_many_arguments)]
pub async fn compensate(
    lib: &Library,
    dev: &LocalDevice,
    rt: &AiRuntime,
    op_id: &str,
    after: AfterUndo,
    now: &Zoned,
    cancel: &Cancel,
    commit_lock: &std::sync::Mutex<()>,
) -> std::result::Result<String, OpError> {
    let target = load(lib, op_id)?;
    if ledger::reverted_ops(&ledger::all(lib)?).contains(op_id) {
        return Ok(String::new()); // undone meanwhile (e.g. on another device)
    }
    let files = op_diff(lib, op_id)?;
    let source = target
        .sources
        .first()
        .and_then(|s| s.parse().ok())
        .map(|id| raw::find(lib, id))
        .transpose()?
        .flatten();
    let source_path = source.as_ref().map(|s| s.path.clone()).unwrap_or_default();
    let fiction = target
        .router
        .as_ref()
        .and_then(|r| r.get("story"))
        .and_then(|s| s.as_str())
        .map(str::to_owned);
    let scope = match fiction {
        Some(s) => Scope::Story(s),
        None => Scope::Personal,
    };
    let new_id = crate::ids::new_id().to_string();
    let mut ctx = OpContext::new(
        lib,
        now.clone(),
        new_id.clone(),
        dev.id.clone(),
        scope,
        source,
    )?;
    ctx.compensating = true;

    let (provider, rc) = rt.for_role(Role::Ingest)?;
    let config = lib.config()?;
    let schema = std::fs::read_to_string(lib.path(crate::layout::SCHEMA_FILE)).unwrap_or_default();
    let system = prompts::render(
        prompts::COMPENSATE,
        &[
            ("today", &now.strftime("%Y-%m-%d").to_string()),
            (
                "timezone",
                now.time_zone().iana_name().unwrap_or("local time"),
            ),
            ("languages", "Persian (fa) and English (en)"),
            (
                "vault_ids",
                &config
                    .active_vaults()
                    .map(|v| v.id.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            ("schema", &schema),
            ("op_id", op_id),
            ("source_path", &source_path),
            ("source_path_no_ext", source_path.trim_end_matches(".md")),
            ("diff", &diff_text(&files)),
        ],
    );
    let pages_list = files
        .iter()
        .map(|f| format!("- {}", f.path))
        .collect::<Vec<_>>()
        .join("\n");
    let user = format!(
        "COMPENSATE task.\nop: {op_id}\nsource path: {source_path}\npages the op touched:\n{pages_list}\n"
    );
    let spec = AgentSpec {
        model: rc.model.clone(),
        system,
        tools: tools::specs(true),
        max_steps: 40,
        max_tokens: 4096,
        temperature: Some(0.0),
        context_chars: 400_000,
        params: rc.params.clone(),
        external: None,
    };
    let validator = move |c: &OpContext<'_>| -> Vec<String> {
        let blame = |p: &str| tools::human_lines_at_head(lib, p);
        validate::validate(lib, &c.cs, &c.scope, &blame).errors
    };
    let outcome = agent::run(
        &provider,
        &spec,
        &mut ctx,
        vec![Message::user(user)],
        Some(&validator),
        2,
        cancel,
        None,
    )
    .await?;

    let _g = commit_lock.lock().unwrap_or_else(|p| p.into_inner());
    if ledger::reverted_ops(&ledger::all(lib)?).contains(op_id) {
        return Ok(String::new());
    }
    let source_paths = status_after(lib, &target, after, &mut ctx.cs)?;
    // Review items the op raised go with it.
    for r in review::list(lib)? {
        if r.op_id.as_deref() == Some(op_id) {
            ctx.cs.files.insert(r.rel_path(), None);
        }
    }
    let mut entry = undo_entry(&target, OpType::Compensate, dev, now, undo_summary(&target));
    entry.op_id = new_id.clone();
    entry.models = vec![
        format!("{}/{}", rc.provider, rc.model),
        prompts::version(prompts::COMPENSATE).to_owned(),
    ];
    entry.usage = outcome.usage.clone();
    entry.usage.cost_usd = rt.config.cost(&rc.model, &outcome.usage);
    changeset::commit(
        lib,
        dev,
        now,
        &ctx.cs,
        entry,
        CommitInfo {
            subject: format!(
                "compensate: undo {} {}",
                target.op_type.as_str(),
                target.op_id
            ),
            source_path: source_paths.first().cloned(),
            log_title: target.summary.clone(),
        },
    )?;
    Ok(new_id)
}
