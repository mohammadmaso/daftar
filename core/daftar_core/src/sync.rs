//! Serverless Git sync (§5) on libgit2.
//!
//! Integration strategy (ADR-0008): local unpushed commits are *rebased* onto the fetched remote
//! head one by one (in-memory cherry-picks), which keeps history linear and lets each local commit
//! be handled by kind:
//!
//! * clean pick → recommitted on top;
//! * AI op commit that conflicts → dropped and returned for **replay** on the merged state;
//! * AI ingest of a source the remote already ingested → the op with the earlier ULID survives
//!   (a local later op is dropped; a remote later op is reverted) — the double-ingest guard;
//! * human / capture commit that conflicts → 3-way text merge, remaining conflicts become a
//!   `> [!conflict]` callout (never Git markers) plus a Review item. Text is never lost.
//!
//! The worktree is always committed before fetching (captures and external edits such as Obsidian),
//! except captures that are not sealed yet (see `raw`).

use std::cell::Cell;
use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use git2::build::CheckoutBuilder;
use git2::{
    Commit, Cred, CredentialType, ErrorClass, ErrorCode, FetchOptions, Index, IndexAddOption, Oid,
    PushOptions, RemoteCallbacks, Repository, Signature, Status, StatusOptions,
};
use jiff::Zoned;
use serde::{Deserialize, Serialize};

use crate::ledger::{self, LedgerEntry, OpType};
use crate::library::{Library, LocalDevice};
use crate::queue::Queue;
use crate::review::{ReviewItem, ReviewKind};
use crate::{Error, Result, layout, raw};

pub const REMOTE: &str = "origin";
pub const MAX_PUSH_ATTEMPTS: usize = 3;

#[derive(Clone, Default)]
pub enum GitAuth {
    #[default]
    None,
    /// HTTPS personal access token. Username defaults to the one in the URL, else `x-access-token`.
    Token {
        username: Option<String>,
        token: String,
    },
    /// OpenSSH private key (PEM text) held in memory, never on disk.
    SshKey {
        private_key: String,
        passphrase: Option<String>,
    },
}

impl std::fmt::Debug for GitAuth {
    // Secrets must never reach logs.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "GitAuth::None",
            Self::Token { .. } => "GitAuth::Token(<redacted>)",
            Self::SshKey { .. } => "GitAuth::SshKey(<redacted>)",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    Synced,
    LocalChanges,
    Offline,
    NeedsAttention,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Replay {
    pub op_id: String,
    pub sources: Vec<String>,
    pub forced_vault: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncOutcome {
    pub state: SyncState,
    pub message: Option<String>,
    pub committed: usize,
    pub pulled: usize,
    pub pushed: usize,
    /// Pages that received a conflict callout.
    pub conflicts: Vec<String>,
    /// Local AI ops dropped during integration that must be re-run on the new state.
    pub replays: Vec<Replay>,
    /// Ops removed by the double-ingest guard.
    pub duplicates_dropped: Vec<String>,
    /// Paths changed by the integration (for incremental re-indexing).
    pub changed_paths: Vec<String>,
    /// Local files not committed because they look like they contain a key or token (§12).
    pub held_back: Vec<String>,
}

impl SyncOutcome {
    fn new(state: SyncState) -> Self {
        Self {
            state,
            message: None,
            committed: 0,
            pulled: 0,
            pushed: 0,
            conflicts: vec![],
            replays: vec![],
            held_back: vec![],
            duplicates_dropped: vec![],
            changed_paths: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalStatus {
    pub uncommitted: usize,
    pub unpushed: usize,
    pub has_remote: bool,
    pub branch: String,
}

// ───────────────────────────── setup ─────────────────────────────

pub(crate) fn signature(dev: &LocalDevice) -> Result<Signature<'static>> {
    let name = if dev.name.is_empty() {
        dev.id.clone()
    } else {
        dev.name.clone()
    };
    Ok(Signature::now(
        &name,
        &format!("{}@{}.local", dev.id, crate::APP_ID),
    )?)
}

fn setup_signature() -> Result<Signature<'static>> {
    Ok(Signature::now(
        crate::APP_NAME,
        &format!("setup@{}.local", crate::APP_ID),
    )?)
}

/// Creates a brand-new local library (no remote yet) with the initial structure committed.
pub fn init_local(root: &Path, branch: &str) -> Result<Library> {
    std::fs::create_dir_all(root)?;
    let mut opts = git2::RepositoryInitOptions::new();
    opts.initial_head(branch);
    let repo = Repository::init_opts(root, &opts)?;
    let lib = Library::at(root);
    lib.write_structure()?;
    commit_all(&repo, "init: new library", &setup_signature()?)?;
    Library::open(root)
}

/// Clones `url` into `root`. An empty remote is initialised with the library structure and pushed.
pub fn clone(url: &str, root: &Path, auth: &GitAuth, branch: &str) -> Result<Library> {
    crate::tls::configure_git(root.parent().unwrap_or(Path::new(".")))?;
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_options(auth));
    let repo = builder.clone(url, root).map_err(classify)?;
    let lib = Library::at(root);
    if repo.head().is_err() || !root.join(layout::CONFIG_FILE).exists() {
        // Empty (or foreign) repository: lay down the structure on the configured branch.
        repo.set_head(&format!("refs/heads/{branch}"))?;
        lib.write_structure()?;
        commit_all(&repo, "init: new library", &setup_signature()?)?;
        push(&repo, branch, auth).map_err(|e| match e {
            PushError::Rejected => Error::NeedsAttention("remote changed during setup".into()),
            PushError::Other(e) => e,
        })?;
    }
    Library::open(root)
}

pub fn set_remote(lib: &Library, url: &str) -> Result<()> {
    let repo = Repository::open(lib.root())?;
    if repo.find_remote(REMOTE).is_ok() {
        repo.remote_set_url(REMOTE, url)?;
    } else {
        repo.remote(REMOTE, url)?;
    }
    Ok(())
}

pub fn remote_url(lib: &Library) -> Result<Option<String>> {
    let repo = Repository::open(lib.root())?;
    Ok(repo
        .find_remote(REMOTE)
        .ok()
        .and_then(|r| r.url().ok().map(str::to_owned)))
}

// ───────────────────────────── committing ─────────────────────────────

fn commit_all(repo: &Repository, message: &str, sig: &Signature<'_>) -> Result<Oid> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    index.update_all(["*"].iter(), None)?;
    index.write()?;
    commit_index(repo, &mut index, message, sig)
}

fn commit_index(
    repo: &Repository,
    index: &mut Index,
    message: &str,
    sig: &Signature<'_>,
) -> Result<Oid> {
    let tree = repo.find_tree(index.write_tree()?)?;
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&Commit<'_>> = parent.iter().collect();
    Ok(repo.commit(Some("HEAD"), sig, sig, message, &tree, &parents)?)
}

/// Stages `paths` (adds, modifications and deletions) and commits them. No-op for an empty set.
pub(crate) fn commit_paths(
    repo: &Repository,
    paths: &[String],
    message: &str,
    sig: &Signature<'_>,
) -> Result<Option<Oid>> {
    if paths.is_empty() {
        return Ok(None);
    }
    let mut index = repo.index()?;
    let workdir = repo.workdir().expect("non-bare").to_path_buf();
    for p in paths {
        if workdir.join(p).exists() {
            index.add_path(Path::new(p))?;
        } else {
            index.remove_path(Path::new(p))?;
        }
    }
    index.write()?;
    commit_index(repo, &mut index, message, sig).map(Some)
}

/// Commits everything in the worktree that is ready: sealed captures (plus their assets and
/// status changes) as one `capture:` commit, and any other change — typically edits made outside
/// the app — as one `edit:` commit. Returns the number of commits created.
fn has_secret(lib: &Library, rel: &str) -> bool {
    (rel.ends_with(".md") || rel.ends_with(".json") || rel.ends_with(".txt"))
        && std::fs::read_to_string(lib.path(rel))
            .is_ok_and(|t| !crate::secrets::scan(&t).is_empty())
}

/// Uncommitted text files that the secret guard is holding back.
pub fn held_back(lib: &Library) -> Result<Vec<String>> {
    let repo = Repository::open(lib.root())?;
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);
    let mut out = Vec::new();
    for s in repo.statuses(Some(&mut opts))?.iter() {
        if let Ok(p) = s.path()
            && has_secret(lib, p)
        {
            out.push(p.to_owned());
        }
    }
    Ok(out)
}

pub fn commit_local(lib: &Library, queue: &Queue, dev: &LocalDevice) -> Result<usize> {
    let repo = Repository::open(lib.root())?;
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);
    let statuses = repo.statuses(Some(&mut opts))?;

    let mut unsealed_assets = HashSet::new();
    let mut captures = Vec::new();
    let mut capture_kinds: Vec<&'static str> = Vec::new();
    let mut raw_other = Vec::new();
    let mut settings = Vec::new();
    let mut edits = Vec::new();

    for s in statuses.iter() {
        let Ok(path) = s.path().map(str::to_owned) else {
            continue;
        };
        if s.status().contains(Status::IGNORED) {
            continue;
        }
        if path.starts_with("raw/assets/") {
            raw_other.push(path);
        } else if path.starts_with("raw/") && path.ends_with(".md") {
            if s.status().contains(Status::WT_NEW) {
                match raw::read(lib, &path) {
                    Ok(item)
                        if queue.awaiting_body(&item.meta.id)?
                            || (item.meta.kind.needs_model_body() && item.body.is_empty()) =>
                    {
                        unsealed_assets.extend(item.meta.assets.iter().cloned());
                    }
                    Ok(item) => {
                        capture_kinds.push(item.meta.kind.as_str());
                        captures.push(path);
                    }
                    Err(_) => edits.push(path),
                }
            } else {
                raw_other.push(path);
            }
        } else if path.starts_with(".daftar/devices/") {
            raw_other.push(path);
        } else if path == layout::CONFIG_FILE {
            settings.push(path);
        } else {
            edits.push(path);
        }
    }
    raw_other.retain(|p| !unsealed_assets.contains(p));
    // §12: anything that looks like a key or token stays out of history until the user redacts it.
    let clean = |p: &String| !has_secret(lib, p);
    captures.retain(clean);
    edits.retain(clean);
    settings.retain(clean);

    let sig = signature(dev)?;
    let mut n = 0;
    if !captures.is_empty() || !raw_other.is_empty() {
        let subject = if captures.is_empty() {
            "sync: capture status and devices".to_owned()
        } else {
            let mut kinds: Vec<_> = capture_kinds.clone();
            kinds.sort();
            kinds.dedup();
            format!(
                "capture: {} {} ({})",
                captures.len(),
                if captures.len() == 1 { "item" } else { "items" },
                kinds.join(", ")
            )
        };
        let mut all = captures;
        all.extend(raw_other);
        let msg = format!("{subject}\n\nDevice: {}\n", dev.id);
        if commit_paths(&repo, &all, &msg, &sig)?.is_some() {
            n += 1;
        }
    }
    if !settings.is_empty() {
        let msg = format!("settings: shared settings changed\n\nDevice: {}\n", dev.id);
        if commit_paths(&repo, &settings, &msg, &sig)?.is_some() {
            n += 1;
        }
    }
    if !edits.is_empty() {
        let shown: Vec<_> = edits.iter().take(5).cloned().collect();
        let more = edits.len().saturating_sub(shown.len());
        let subject = format!(
            "edit: {}{}",
            shown.join(", "),
            if more > 0 {
                format!(" (+{more})")
            } else {
                String::new()
            }
        );
        let msg = format!("{subject}\n\nEdit-Source: external\nDevice: {}\n", dev.id);
        if commit_paths(&repo, &edits, &msg, &sig)?.is_some() {
            n += 1;
        }
    }
    Ok(n)
}

// ───────────────────────────── status ─────────────────────────────

pub fn local_status(lib: &Library) -> Result<LocalStatus> {
    let repo = Repository::open(lib.root())?;
    let branch = current_branch(&repo)?;
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let uncommitted = repo.statuses(Some(&mut opts))?.len();
    let has_remote = repo.find_remote(REMOTE).is_ok();
    let unpushed = match (
        repo.head().ok().and_then(|h| h.target()),
        upstream_oid(&repo, &branch),
    ) {
        (Some(local), Some(up)) => repo.graph_ahead_behind(local, up)?.0,
        (Some(local), None) => count_commits(&repo, local)?,
        _ => 0,
    };
    Ok(LocalStatus {
        uncommitted,
        unpushed,
        has_remote,
        branch,
    })
}

fn count_commits(repo: &Repository, from: Oid) -> Result<usize> {
    let mut walk = repo.revwalk()?;
    walk.push(from)?;
    Ok(walk.count())
}

fn current_branch(repo: &Repository) -> Result<String> {
    let head = repo.find_reference("HEAD")?;
    let target = head
        .symbolic_target()
        .ok()
        .flatten()
        .unwrap_or("refs/heads/main");
    Ok(target.trim_start_matches("refs/heads/").to_owned())
}

fn upstream_oid(repo: &Repository, branch: &str) -> Option<Oid> {
    repo.refname_to_id(&format!("refs/remotes/{REMOTE}/{branch}"))
        .ok()
}

// ───────────────────────────── network ─────────────────────────────

fn callbacks(auth: &GitAuth) -> RemoteCallbacks<'_> {
    let mut cb = RemoteCallbacks::new();
    let tries = Cell::new(0u8);
    cb.credentials(move |_url, user_from_url, allowed| {
        // libgit2 retries the callback on failure; give each method one chance.
        tries.set(tries.get() + 1);
        if tries.get() > 2 {
            return Err(git2::Error::new(
                ErrorCode::Auth,
                ErrorClass::Http,
                "credentials rejected",
            ));
        }
        match auth {
            GitAuth::Token { username, token }
                if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) =>
            {
                let user = username
                    .as_deref()
                    .or(user_from_url)
                    .unwrap_or("x-access-token");
                Cred::userpass_plaintext(user, token)
            }
            GitAuth::SshKey {
                private_key,
                passphrase,
            } if allowed.contains(CredentialType::SSH_KEY) => Cred::ssh_key_from_memory(
                user_from_url.unwrap_or("git"),
                None,
                private_key,
                passphrase.as_deref(),
            ),
            _ if allowed.contains(CredentialType::USERNAME) => {
                Cred::username(user_from_url.unwrap_or("git"))
            }
            _ => Cred::default(),
        }
    });
    cb
}

fn proxy_options() -> git2::ProxyOptions<'static> {
    // Honour git config / environment proxies (common behind censorship and corporate networks).
    let mut p = git2::ProxyOptions::new();
    p.auto();
    p
}

fn fetch_options(auth: &GitAuth) -> FetchOptions<'_> {
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks(auth));
    fo.proxy_options(proxy_options());
    fo
}

/// Maps libgit2 errors to user-meaningful categories.
fn classify(e: git2::Error) -> Error {
    let msg = e.message().to_lowercase();
    if e.code() == ErrorCode::Auth
        || msg.contains("401")
        || msg.contains("403")
        || msg.contains("authentication")
        || msg.contains("credentials")
        || msg.contains("permission denied")
    {
        return Error::Auth(e.message().to_owned());
    }
    match e.class() {
        ErrorClass::Net | ErrorClass::Ssl | ErrorClass::Os | ErrorClass::Http | ErrorClass::Ssh => {
            Error::Offline
        }
        // Local-path remotes that disappear (tests, unplugged drives) behave like being offline.
        ErrorClass::Repository
            if msg.contains("could not find repository") || msg.contains("failed to resolve") =>
        {
            Error::Offline
        }
        _ => Error::Git(e),
    }
}

fn fetch(repo: &Repository, branch: &str, auth: &GitAuth) -> Result<Option<Oid>> {
    let mut remote = repo.find_remote(REMOTE)?;
    let refspec = format!("+refs/heads/{branch}:refs/remotes/{REMOTE}/{branch}");
    remote
        .fetch(&[&refspec], Some(&mut fetch_options(auth)), None)
        .map_err(classify)?;
    Ok(upstream_oid(repo, branch))
}

enum PushError {
    Rejected,
    Other(Error),
}

fn push(repo: &Repository, branch: &str, auth: &GitAuth) -> std::result::Result<(), PushError> {
    let mut remote = repo
        .find_remote(REMOTE)
        .map_err(|e| PushError::Other(e.into()))?;
    let rejected = Cell::new(false);
    let mut cb = callbacks(auth);
    cb.push_update_reference(|_r, status| {
        if status.is_some() {
            rejected.set(true);
        }
        Ok(())
    });
    let mut po = PushOptions::new();
    po.remote_callbacks(cb);
    po.proxy_options(proxy_options());
    let spec = format!("refs/heads/{branch}:refs/heads/{branch}");
    match remote.push(&[&spec], Some(&mut po)) {
        Ok(()) if !rejected.get() => {}
        Ok(()) => return Err(PushError::Rejected),
        Err(e) => {
            let m = e.message().to_lowercase();
            return Err(
                if m.contains("non-fast-forward")
                    || m.contains("fast-forward")
                    || m.contains("contains commits")
                    || e.code() == ErrorCode::NotFastForward
                {
                    PushError::Rejected
                } else {
                    PushError::Other(classify(e))
                },
            );
        }
    }
    // Keep the remote-tracking ref in step so ahead/behind is correct without another fetch.
    if let Ok(head) = repo.refname_to_id(&format!("refs/heads/{branch}")) {
        let _ = repo.reference(
            &format!("refs/remotes/{REMOTE}/{branch}"),
            head,
            true,
            "push",
        );
    }
    Ok(())
}

// ───────────────────────────── integration ─────────────────────────────

struct Pick<'r> {
    commit: Commit<'r>,
    op: Option<LedgerEntry>,
}

/// Ledger entry introduced by a commit, if it is an AI op commit.
fn op_of(repo: &Repository, lib: &Library, c: &Commit<'_>) -> Option<LedgerEntry> {
    let msg = c.message().unwrap_or_default();
    let op_id = ledger::trailer(msg, "Op-Id")?;
    // Read the ledger file from the commit's own tree: it may not be in the worktree any more.
    let id: ulid::Ulid = op_id.parse().ok()?;
    let ts = jiff::Timestamp::from_millisecond(id.timestamp_ms() as i64)
        .ok()?
        .to_zoned(jiff::tz::TimeZone::UTC);
    let rel = layout::ledger_entry(crate::time::date_of(&ts), id);
    let entry = c.tree().ok()?.get_path(Path::new(&rel)).ok()?;
    let blob = repo.find_blob(entry.id()).ok()?;
    let _ = lib;
    serde_json::from_slice(blob.content()).ok()
}

fn ledger_at(repo: &Repository, commit: &Commit<'_>) -> Result<Vec<LedgerEntry>> {
    let mut out = Vec::new();
    let tree = commit.tree()?;
    let Ok(dir) = tree.get_path(Path::new(".daftar/ledger")) else {
        return Ok(out);
    };
    let dir = repo.find_tree(dir.id())?;
    dir.walk(git2::TreeWalkMode::PreOrder, |_, e| {
        if e.name().is_ok_and(|n| n.ends_with(".json"))
            && let Ok(b) = repo.find_blob(e.id())
            && let Ok(entry) = serde_json::from_slice::<LedgerEntry>(b.content())
        {
            out.push(entry);
        }
        git2::TreeWalkResult::Ok
    })?;
    out.sort_by(|a, b| a.op_id.cmp(&b.op_id));
    Ok(out)
}

/// Synchronises with the remote. Never fails for being offline; returns the state instead.
pub fn sync(
    lib: &Library,
    queue: &Queue,
    dev: &LocalDevice,
    auth: &GitAuth,
    now: &Zoned,
) -> Result<SyncOutcome> {
    crate::tls::configure_git(&lib.local_dir())?;
    let committed = commit_local(lib, queue, dev)?;
    let held = held_back(lib)?;
    let repo = Repository::open(lib.root())?;
    let branch = current_branch(&repo)?;
    if repo.find_remote(REMOTE).is_err() {
        let mut o = SyncOutcome::new(SyncState::LocalChanges);
        o.committed = committed;
        o.held_back = held;
        o.message = Some("no remote configured".into());
        return Ok(o);
    }

    let mut outcome = SyncOutcome::new(SyncState::Synced);
    outcome.committed = committed;
    outcome.held_back = held;
    for attempt in 1..=MAX_PUSH_ATTEMPTS {
        let upstream = match fetch(&repo, &branch, auth) {
            Ok(u) => u,
            Err(Error::Offline) => {
                outcome.state = SyncState::Offline;
                return Ok(outcome);
            }
            Err(Error::Auth(m)) => {
                outcome.state = SyncState::NeedsAttention;
                outcome.message = Some(format!("The remote rejected the credentials: {m}"));
                return Ok(outcome);
            }
            Err(e) => return Err(e),
        };
        let head = repo.head()?.peel_to_commit()?;
        if let Some(up) = upstream {
            integrate(lib, &repo, &branch, dev, now, head.id(), up, &mut outcome)?;
            regenerate_indexes_after_merge(lib, &repo, dev, &outcome.changed_paths)?;
        }
        let head = repo.head()?.peel_to_commit()?;
        let ahead = match upstream_oid(&repo, &branch) {
            Some(up) => repo.graph_ahead_behind(head.id(), up)?.0,
            None => count_commits(&repo, head.id())?,
        };
        if ahead == 0 {
            outcome.state = SyncState::Synced;
            return Ok(outcome);
        }
        match push(&repo, &branch, auth) {
            Ok(()) => {
                outcome.pushed += ahead;
                outcome.state = SyncState::Synced;
                return Ok(outcome);
            }
            Err(PushError::Rejected) if attempt < MAX_PUSH_ATTEMPTS => continue,
            Err(PushError::Rejected) => {
                outcome.state = SyncState::NeedsAttention;
                outcome.message =
                    Some("The remote kept changing while pushing. Try again in a moment.".into());
                return Ok(outcome);
            }
            Err(PushError::Other(Error::Offline)) => {
                outcome.state = SyncState::Offline;
                return Ok(outcome);
            }
            Err(PushError::Other(Error::Auth(m))) => {
                outcome.state = SyncState::NeedsAttention;
                outcome.message = Some(format!("The remote refused the push: {m}"));
                return Ok(outcome);
            }
            Err(PushError::Other(e)) => return Err(e),
        }
    }
    unreachable!("loop returns on every path")
}

#[allow(clippy::too_many_arguments)]
fn integrate(
    lib: &Library,
    repo: &Repository,
    branch: &str,
    dev: &LocalDevice,
    now: &Zoned,
    local: Oid,
    upstream: Oid,
    outcome: &mut SyncOutcome,
) -> Result<()> {
    if local == upstream || repo.graph_descendant_of(local, upstream)? {
        return Ok(()); // nothing new remotely
    }
    let base = repo.merge_base(local, upstream).ok();
    let behind = repo.graph_ahead_behind(local, upstream)?.1;

    // Local commits not on the remote, oldest first.
    let mut picks = Vec::new();
    if base != Some(local) {
        let mut walk = repo.revwalk()?;
        walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::REVERSE)?;
        walk.push(local)?;
        if let Some(b) = base {
            walk.hide(b)?;
        }
        for oid in walk {
            let c = repo.find_commit(oid?)?;
            let op = op_of(repo, lib, &c);
            picks.push(Pick { commit: c, op });
        }
    }

    let up_commit = repo.find_commit(upstream)?;
    let remote_ledger = ledger_at(repo, &up_commit)?;
    let remote_live = ledger::live_ingests_by_source(&remote_ledger);

    // Double-ingest guard (§5.4 step 5): for each source ingested on both sides, the op with the
    // earlier ULID survives. Decide everything up front so remote losers are reverted *before*
    // local winners are replayed on top of them.
    let mut dropped_local: HashSet<String> = HashSet::new();
    let mut revert_remote: BTreeSet<String> = BTreeSet::new();
    for pick in &picks {
        let Some(op) = pick.op.as_ref().filter(|o| o.op_type == OpType::Ingest) else {
            continue;
        };
        for s in &op.sources {
            for r in remote_live.get(s).into_iter().flatten() {
                if r < &op.op_id {
                    dropped_local.insert(op.op_id.clone());
                } else {
                    revert_remote.insert(r.clone());
                }
            }
        }
    }

    let mut tip = up_commit;
    for op_id in revert_remote {
        let Some(target) = find_op_commit(repo, upstream, &op_id)? else {
            continue;
        };
        let mut idx = repo.revert_commit(&target, &tip, 0, None)?;
        if idx.has_conflicts() {
            // Later remote work builds on it; the local op then replays and its pre-ingest check
            // sees the remote op as already filed, so exactly one op still survives.
            continue;
        }
        let tree = repo.find_tree(idx.write_tree_to(repo)?)?;
        let entry = revert_entry(&op_id, dev, now);
        let mut json = serde_json::to_string_pretty(&entry)?;
        json.push('\n');
        let tree = with_file(repo, &tree, &entry.rel_path(), json.as_bytes())?;
        let msg = ledger::commit_message("revert-op: duplicate ingest", &entry, None, &[]);
        let sig = signature(dev)?;
        let oid = repo.commit(None, &sig, &sig, &msg, &tree, &[&tip])?;
        tip = repo.find_commit(oid)?;
        outcome.duplicates_dropped.push(op_id);
    }

    for pick in picks {
        if let Some(op) = &pick.op
            && dropped_local.contains(&op.op_id)
        {
            outcome.duplicates_dropped.push(op.op_id.clone());
            continue;
        }
        let mut idx = repo.cherrypick_commit(&pick.commit, &tip, 0, None)?;
        if idx.has_conflicts() {
            if let Some(op) = &pick.op {
                // AI op: drop it and replay on top of the merged state (§5.4 step 3).
                outcome.replays.push(Replay {
                    op_id: op.op_id.clone(),
                    sources: op.sources.clone(),
                    forced_vault: op.forced_vault.clone(),
                    note: op.note.clone(),
                });
                continue;
            }
            let other = tip.author().name().unwrap_or("another device").to_owned();
            resolve_human_conflicts(lib, repo, &mut idx, dev, &other, now, outcome)?;
        }
        let tree = repo.find_tree(idx.write_tree_to(repo)?)?;
        if tree.id() == tip.tree_id() {
            continue; // became empty (change already upstream)
        }
        let c = &pick.commit;
        let committer = signature(dev)?;
        let oid = repo.commit(
            None,
            &c.author(),
            &committer,
            c.message().unwrap_or(""),
            &tree,
            &[&tip],
        )?;
        tip = repo.find_commit(oid)?;
    }

    // Move the worktree and branch to the new tip. Safe checkout refuses to clobber edits made
    // since `commit_local`; in that case the next sync picks them up.
    let old_tree = repo.find_commit(local)?.tree()?;
    let new_tree = tip.tree()?;
    let diff = repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?;
    for d in diff.deltas() {
        if let Some(p) = d.new_file().path().or(d.old_file().path()) {
            outcome
                .changed_paths
                .push(p.to_string_lossy().replace('\\', "/"));
        }
    }
    let mut co = CheckoutBuilder::new();
    co.safe();
    repo.checkout_tree(tip.as_object(), Some(&mut co))?;
    repo.reference(
        &format!("refs/heads/{branch}"),
        tip.id(),
        true,
        "daftar sync",
    )?;
    outcome.pulled += behind;
    Ok(())
}

/// Generated indexes are never merged (§4.4): rebuild those of vaults whose pages changed.
fn regenerate_indexes_after_merge(
    lib: &Library,
    repo: &Repository,
    dev: &LocalDevice,
    changed: &[String],
) -> Result<()> {
    let vaults: BTreeSet<String> = changed
        .iter()
        .filter(|p| !crate::pages::is_generated_index(p))
        .filter_map(|p| crate::pages::vault_of(p).map(str::to_owned))
        .collect();
    if vaults.is_empty() {
        return Ok(());
    }
    let paths = crate::pages::regenerate_indexes(lib, &vaults.into_iter().collect::<Vec<_>>())?;
    commit_paths(
        repo,
        &paths,
        &format!("index: regenerate\n\nDevice: {}\n", dev.id),
        &signature(dev)?,
    )?;
    Ok(())
}

fn revert_entry(target: &str, dev: &LocalDevice, now: &Zoned) -> LedgerEntry {
    let ts = crate::time::rfc3339(now);
    LedgerEntry {
        op_id: ulid::Ulid::generate().to_string(),
        op_type: OpType::RevertOp,
        sources: vec![],
        router: None,
        models: vec![],
        pages_created: vec![],
        pages_updated: vec![],
        claims_added: vec![],
        review_items: vec![],
        usage: Default::default(),
        started_at: ts.clone(),
        finished_at: ts,
        device: dev.id.clone(),
        summary: "Removed a duplicate filing of the same capture.".into(),
        note: None,
        forced_vault: None,
        replayed_from: None,
        reverts: Some(target.to_owned()),
        rejected_claims: vec![],
    }
}

pub(crate) fn find_op_commit<'r>(
    repo: &'r Repository,
    from: Oid,
    op_id: &str,
) -> Result<Option<Commit<'r>>> {
    let mut walk = repo.revwalk()?;
    walk.push(from)?;
    for oid in walk {
        let c = repo.find_commit(oid?)?;
        if ledger::trailer(c.message().unwrap_or_default(), "Op-Id") == Some(op_id) {
            return Ok(Some(c));
        }
    }
    Ok(None)
}

/// Returns `tree` with `rel` set to `content` (creating intermediate trees).
pub(crate) fn with_file<'r>(
    repo: &'r Repository,
    tree: &git2::Tree<'r>,
    rel: &str,
    content: &[u8],
) -> Result<git2::Tree<'r>> {
    let mut idx = Index::new()?;
    idx.read_tree(tree)?;
    let blob = repo.blob(content)?;
    let entry = git2::IndexEntry {
        ctime: git2::IndexTime::new(0, 0),
        mtime: git2::IndexTime::new(0, 0),
        dev: 0,
        ino: 0,
        mode: 0o100644,
        uid: 0,
        gid: 0,
        file_size: content.len() as u32,
        id: blob,
        flags: 0,
        flags_extended: 0,
        path: rel.as_bytes().to_vec(),
    };
    idx.add(&entry)?;
    Ok(repo.find_tree(idx.write_tree_to(repo)?)?)
}

fn resolve_human_conflicts(
    lib: &Library,
    repo: &Repository,
    idx: &mut Index,
    dev: &LocalDevice,
    other_device: &str,
    now: &Zoned,
    outcome: &mut SyncOutcome,
) -> Result<()> {
    let conflicts: Vec<_> = idx.conflicts()?.collect::<std::result::Result<_, _>>()?;
    for c in conflicts {
        let path_bytes = c
            .our
            .as_ref()
            .or(c.their.as_ref())
            .or(c.ancestor.as_ref())
            .map(|e| e.path.clone())
            .unwrap_or_default();
        let path = String::from_utf8_lossy(&path_bytes).into_owned();
        let resolved: Option<Vec<u8>> = match (&c.ancestor, &c.our, &c.their) {
            (_, Some(ours), Some(theirs)) => {
                let is_generated = path.starts_with("vaults/")
                    && path.ends_with("/index.md")
                    && path.matches('/').count() == 2;
                if is_generated {
                    Some(repo.find_blob(ours.id)?.content().to_vec())
                } else if path == layout::CONFIG_FILE {
                    let parse = |e: &git2::IndexEntry| -> Result<serde_json::Value> {
                        Ok(serde_json::from_slice(repo.find_blob(e.id)?.content())?)
                    };
                    let base = c.ancestor.as_ref().map(parse).transpose()?;
                    // `ours` is the remote side here, `theirs` this device (see labels below).
                    let merged =
                        crate::config::merge_json(base.as_ref(), &parse(ours)?, &parse(theirs)?);
                    let mut text = serde_json::to_string_pretty(&merged)?;
                    text.push('\n');
                    Some(text.into_bytes())
                } else {
                    let mut opts = git2::MergeFileOptions::new();
                    opts.our_label("remote").their_label("local");
                    let merged = repo.merge_file_from_index(
                        c.ancestor.as_ref().unwrap_or(ours),
                        ours,
                        theirs,
                        Some(&mut opts),
                    )?;
                    let text = String::from_utf8_lossy(merged.content()).into_owned();
                    if merged.is_automergeable() {
                        Some(text.into_bytes())
                    } else {
                        outcome.conflicts.push(path.clone());
                        let item = ReviewItem::new(
                            ReviewKind::SyncConflict,
                            &dev.id,
                            now,
                            None,
                            serde_json::json!({"path": path, "devices": [other_device, dev.name]}),
                        );
                        let rel = item.rel_path();
                        let mut json = serde_json::to_string_pretty(&item)?;
                        json.push('\n');
                        add_blob(repo, idx, &rel, json.as_bytes())?;
                        let _ = lib;
                        Some(
                            markers_to_callouts(&text, &dev.name, &crate::time::rfc3339(now)[..10])
                                .into_bytes(),
                        )
                    }
                }
            }
            // Modified on one side, deleted on the other: keep the modified version.
            (_, Some(kept), None) | (_, None, Some(kept)) => {
                Some(repo.find_blob(kept.id)?.content().to_vec())
            }
            _ => None,
        };
        idx.remove_path(Path::new(&path))?;
        if let Some(bytes) = resolved {
            add_blob(repo, idx, &path, &bytes)?;
        }
    }
    Ok(())
}

fn add_blob(repo: &Repository, idx: &mut Index, rel: &str, content: &[u8]) -> Result<()> {
    let blob = repo.blob(content)?;
    let entry = git2::IndexEntry {
        ctime: git2::IndexTime::new(0, 0),
        mtime: git2::IndexTime::new(0, 0),
        dev: 0,
        ino: 0,
        mode: 0o100644,
        uid: 0,
        gid: 0,
        file_size: content.len() as u32,
        id: blob,
        flags: 0,
        flags_extended: 0,
        path: rel.as_bytes().to_vec(),
    };
    idx.add(&entry)?;
    Ok(())
}

/// Rewrites Git conflict markers into Obsidian callouts: the remote version stays in place, the
/// local version follows in a `[!conflict]` callout. No marker ever reaches the file.
pub fn markers_to_callouts(text: &str, local_device: &str, date: &str) -> String {
    #[derive(PartialEq)]
    enum S {
        Normal,
        Ours,
        Theirs,
    }
    let mut out = String::with_capacity(text.len() + 128);
    let mut state = S::Normal;
    let mut theirs: Vec<&str> = Vec::new();
    for line in text.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\r', '\n']);
        match state {
            S::Normal if bare.starts_with("<<<<<<<") => state = S::Ours,
            S::Ours if bare.starts_with("=======") => state = S::Theirs,
            S::Ours if bare.starts_with("|||||||") => {}
            S::Theirs if bare.starts_with(">>>>>>>") => {
                if !out.is_empty() && !out.ends_with("\n\n") {
                    if !out.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push('\n');
                }
                out.push_str(&format!("> [!conflict] From {local_device} · {date}\n"));
                for t in theirs.drain(..) {
                    let t = t.trim_end_matches(['\r', '\n']);
                    if t.is_empty() {
                        out.push_str(">\n");
                    } else {
                        out.push_str(&format!("> {t}\n"));
                    }
                }
                out.push('\n');
                state = S::Normal;
            }
            S::Normal | S::Ours => out.push_str(line),
            S::Theirs => theirs.push(line),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::markers_to_callouts;

    #[test]
    fn markers_become_callouts() {
        let t = "# Sara\n<<<<<<< remote\nSara lives in Tehran.\n=======\nSara lives in Shiraz.\n>>>>>>> local\nEnd.\n";
        let out = markers_to_callouts(t, "pixel-8", "2026-09-23");
        assert!(!out.contains("<<<<<<<") && !out.contains("=======") && !out.contains(">>>>>>>"));
        assert_eq!(
            out,
            "# Sara\nSara lives in Tehran.\n\n> [!conflict] From pixel-8 · 2026-09-23\n> Sara lives in Shiraz.\n\nEnd.\n"
        );
    }
}
