//! Staged, validated, atomic AI writes (§6.4). The model never touches files: tools stage edits
//! here; the validator checks them; `commit` applies everything as one Git commit + one ledger entry.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use git2::Repository;
use jiff::Zoned;
use serde::{Deserialize, Serialize};

use crate::fsutil::atomic_write;
use crate::ledger::{self, LedgerEntry};
use crate::library::{Library, LocalDevice};
use crate::raw::{self, RawStatus};
use crate::review::{self, ReviewItem};
use crate::{Result, layout, pages};

#[derive(Debug, Clone, Default)]
pub struct Changeset {
    /// path → new content; `None` = delete.
    pub files: BTreeMap<String, Option<String>>,
    pub created: BTreeSet<String>,
    pub claims_added: Vec<String>,
    /// Claim ids produced by claim tools (anything else with `^c-` is rejected).
    pub claim_ids_from_tools: BTreeSet<String>,
    pub review_items: Vec<ReviewItem>,
    /// Raw captures whose status must change when this op commits.
    pub raw_status: BTreeMap<String, RawStatus>,
}

impl Changeset {
    pub fn read(&self, lib: &Library, path: &str) -> Option<String> {
        match self.files.get(path) {
            Some(Some(s)) => Some(s.clone()),
            Some(None) => None,
            None => fs::read_to_string(lib.path(path)).ok(),
        }
    }

    pub fn original(&self, lib: &Library, path: &str) -> Option<String> {
        fs::read_to_string(lib.path(path)).ok()
    }

    pub fn exists(&self, lib: &Library, path: &str) -> bool {
        match self.files.get(path) {
            Some(v) => v.is_some(),
            None => lib.path(path).is_file(),
        }
    }

    pub fn write(&mut self, lib: &Library, path: &str, content: String) {
        if !lib.path(path).exists() {
            self.created.insert(path.to_owned());
        }
        self.files.insert(path.to_owned(), Some(content));
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.review_items.is_empty()
    }

    /// Pages created/updated (repo paths), for the ledger.
    pub fn page_changes(&self) -> (Vec<String>, Vec<String>) {
        let mut created = Vec::new();
        let mut updated = Vec::new();
        for p in self.files.keys().filter(|p| p.starts_with("vaults/")) {
            if self.created.contains(p) {
                created.push(p.clone())
            } else {
                updated.push(p.clone())
            }
        }
        (created, updated)
    }

    pub fn touched_vaults(&self) -> Vec<String> {
        let v: BTreeSet<String> = self
            .files
            .keys()
            .filter_map(|p| pages::vault_of(p).map(str::to_owned))
            .collect();
        v.into_iter().collect()
    }
}

/// Written before files are applied and removed after the commit; lets a crash mid-apply be rolled
/// back so half-applied AI writes are never committed as human edits.
#[derive(Debug, Serialize, Deserialize)]
struct PendingOp {
    op_id: String,
    paths: Vec<String>,
}

fn pending_path(lib: &Library) -> std::path::PathBuf {
    lib.local_dir().join("pending-op.json")
}

/// Undoes a half-applied op after a crash. Returns the op id that was rolled back, if any.
pub fn recover(lib: &Library) -> Result<Option<String>> {
    let p = pending_path(lib);
    let Ok(bytes) = fs::read(&p) else {
        return Ok(None);
    };
    let pending: PendingOp = serde_json::from_slice(&bytes)?;
    let repo = Repository::open(lib.root())?;
    let committed = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .is_some_and(|c| {
            ledger::trailer(c.message().unwrap_or_default(), "Op-Id")
                == Some(pending.op_id.as_str())
        });
    if !committed {
        let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
        for path in &pending.paths {
            match head_tree
                .as_ref()
                .and_then(|t| t.get_path(Path::new(path)).ok())
            {
                Some(entry) => {
                    let blob = repo.find_blob(entry.id())?;
                    atomic_write(&lib.path(path), blob.content())?;
                }
                None => {
                    let _ = fs::remove_file(lib.path(path));
                }
            }
        }
    }
    fs::remove_file(&p)?;
    Ok((!committed).then_some(pending.op_id))
}

pub struct CommitInfo {
    pub subject: String,
    pub source_path: Option<String>,
    pub log_title: String,
}

/// Applies the changeset, updates raw status, log, ledger and indexes, and commits once.
pub fn commit(
    lib: &Library,
    dev: &LocalDevice,
    now: &Zoned,
    cs: &Changeset,
    mut entry: LedgerEntry,
    info: CommitInfo,
) -> Result<git2::Oid> {
    // A claim proposed and then removed within the same op leaves no card behind.
    let pruned;
    let cs = {
        let alive = |page: &str, id: &str| {
            cs.read(lib, page)
                .is_some_and(|t| t.lines().any(|l| l.trim_end().ends_with(&format!("^{id}"))))
        };
        let mut c = cs.clone();
        c.review_items.retain(|r| {
            r.kind != review::ReviewKind::Claim
                || alive(
                    r.payload["page"].as_str().unwrap_or_default(),
                    r.payload["claim_id"].as_str().unwrap_or_default(),
                )
        });
        let pages: Vec<String> = c.files.keys().cloned().collect();
        c.claims_added
            .retain(|id| pages.iter().any(|p| alive(p, id)));
        pruned = c;
        &pruned
    };
    let (created, updated) = cs.page_changes();
    entry.pages_created = created;
    entry.pages_updated = updated;
    entry.claims_added = cs.claims_added.clone();
    entry.review_items = cs.review_items.iter().map(|r| r.id.clone()).collect();

    let mut paths: BTreeSet<String> = cs.files.keys().cloned().collect();
    let ledger_rel = entry.rel_path();
    paths.insert(ledger_rel.clone());
    let log_rel = layout::monthly_log(crate::time::date_of(now));
    paths.insert(log_rel.clone());
    for r in &cs.review_items {
        paths.insert(r.rel_path());
    }
    let vaults = cs.touched_vaults();
    for v in &vaults {
        paths.insert(layout::vault_index(v));
    }
    for raw_path in cs.raw_status.keys() {
        paths.insert(raw_path.clone());
    }

    // Raw captures must never live only inside an op commit: a replay drops op commits, and the
    // capture would vanish with it. Commit any not-yet-committed capture on its own first.
    let repo = Repository::open(lib.root())?;
    let sig = git2::Signature::now(&dev.name, &format!("{}@{}.local", dev.id, crate::APP_ID))?;
    let untracked: Vec<String> = cs
        .raw_status
        .keys()
        .filter(|p| {
            repo.status_file(Path::new(p))
                .is_ok_and(|s| s.contains(git2::Status::WT_NEW))
        })
        .cloned()
        .collect();
    if let Some(p) = untracked.iter().find(|p| {
        std::fs::read_to_string(lib.path(p)).is_ok_and(|t| !crate::secrets::scan(&t).is_empty())
    }) {
        return Err(crate::Error::invalid(format!(
            "{p} looks like it contains a key or token; redact it before it is filed."
        )));
    }
    if !untracked.is_empty() {
        let mut with_assets = untracked.clone();
        for p in &untracked {
            if let Ok(item) = raw::read(lib, p) {
                with_assets.extend(item.meta.assets.into_iter().filter(|a| {
                    repo.status_file(Path::new(a))
                        .is_ok_and(|s| s.contains(git2::Status::WT_NEW))
                }));
            }
        }
        let msg = format!(
            "capture: {} {}\n\nDevice: {}\n",
            untracked.len(),
            if untracked.len() == 1 {
                "item"
            } else {
                "items"
            },
            dev.id
        );
        crate::sync::commit_paths(&repo, &with_assets, &msg, &sig)?;
    }

    let pending = PendingOp {
        op_id: entry.op_id.clone(),
        paths: paths.iter().cloned().collect(),
    };
    atomic_write(&pending_path(lib), &serde_json::to_vec(&pending)?)?;

    for (path, content) in &cs.files {
        match content {
            Some(c) => atomic_write(&lib.path(path), c.as_bytes())?,
            None => {
                let _ = fs::remove_file(lib.path(path));
            }
        }
    }
    for (raw_path, status) in &cs.raw_status {
        raw::set_status(lib, raw_path, *status)?;
    }
    for r in &cs.review_items {
        review::add(lib, r)?;
    }
    ledger::write(lib, &entry)?;
    append_log(lib, &log_rel, now, &entry, &vaults, &info.log_title)?;
    pages::regenerate_indexes(lib, &vaults)?;

    let msg = ledger::commit_message(&info.subject, &entry, info.source_path.as_deref(), &vaults);
    let oid = crate::sync::commit_paths(&repo, &paths.into_iter().collect::<Vec<_>>(), &msg, &sig)?
        .expect("non-empty");
    fs::remove_file(pending_path(lib))?;
    Ok(oid)
}

fn append_log(
    lib: &Library,
    rel: &str,
    now: &Zoned,
    entry: &LedgerEntry,
    vaults: &[String],
    title: &str,
) -> Result<()> {
    let mut text = fs::read_to_string(lib.path(rel))
        .unwrap_or_else(|_| format!("# Log · {}\n\n", now.strftime("%Y-%m")));
    if !text.ends_with('\n') {
        text.push('\n');
    }
    let short: String = title.chars().take(80).collect();
    text.push_str(&format!(
        "## [{}] {} | {} | \"{}\" (op {})\n",
        now.strftime("%Y-%m-%d %H:%M"),
        entry.op_type.as_str(),
        if vaults.is_empty() {
            "-".to_owned()
        } else {
            vaults.join(", ")
        },
        short.replace('"', "'"),
        entry.op_id
    ));
    atomic_write(&lib.path(rel), text.as_bytes())?;
    Ok(())
}
