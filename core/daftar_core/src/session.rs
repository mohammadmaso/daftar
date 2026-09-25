//! Orchestration shared by the app (through the bridge), the CLI and scenario tests: one open
//! library on this device with its job queue.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use jiff::Zoned;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::Cancel;
use crate::audit::{AfterUndo, OpSummary, UndoOutcome};
use crate::layout::Date;
use crate::library::{Library, LocalDevice};
use crate::ops::{self, IngestOptions, IngestOutcome, OpError};
use crate::providers::{ProviderErrorKind, Role};
use crate::queue::{JobKind, JobState, Queue};
use crate::raw::{self, NewCapture, RawItem, RawKind, RawStatus};
use crate::runtime::AiRuntime;
use crate::search::{Graph, Hit, PageRef, SearchIndex};
use crate::sync::{self, GitAuth, LocalStatus, SyncOutcome};
use crate::{Result, assets};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Session {
    lib: Library,
    device: LocalDevice,
    queue: Mutex<Queue>,
    sync_lock: Mutex<()>,
    /// Search/link cache, opened on first use and refreshed when the repository may have changed.
    index: Mutex<Option<SearchIndex>>,
    index_dirty: AtomicBool,
}

/// Result of an undo request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UndoState {
    /// Reverted; the id of the `revert-op`.
    Done(String),
    /// Later changes overlap; a compensating op will run with the AI jobs.
    Queued,
}

/// A page opened in the reader or editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageView {
    pub path: String,
    /// The whole file, for the source editor.
    pub text: String,
    /// Markdown after the frontmatter, for the renderer.
    pub body: String,
    /// Base for `save_page`.
    pub hash: String,
    pub meta: crate::wiki::PageMeta,
    pub backlinks: Vec<PageRef>,
}

/// Where a capture is in its life, as shown in the Today timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureStage {
    /// Saved locally; waiting for transcription / description / filing.
    Saved,
    /// A job for it is running right now.
    Working,
    /// A job for it failed and needs the user (see `problem`).
    Failed,
    /// Filed into the wiki.
    Filed,
    /// Excluded by the user.
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureView {
    pub id: String,
    pub path: String,
    pub kind: RawKind,
    pub captured_at: String,
    pub device: String,
    pub text: String,
    pub vault_hint: Option<String>,
    /// Absolute paths of image assets, for thumbnails.
    pub images: Vec<String>,
    pub stage: CaptureStage,
    pub problem: Option<String>,
    /// What filing did, once filed (§4.2 step 5).
    pub filing: Option<Filing>,
}

/// The outcome of the live ingest op for a capture, for "Filed to Life · Health — 4 pages
/// updated, 1 claim to review".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Filing {
    pub op_id: String,
    pub vaults: Vec<String>,
    pub pages_created: usize,
    pub pages_updated: usize,
    /// Proposed claims from this op still waiting in Review.
    pub claims_to_review: usize,
    /// All Review items from this op still waiting (claims, routing, questions).
    pub to_review: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobReport {
    pub job_id: String,
    pub kind: JobKind,
    pub raw_id: Option<String>,
    pub state: JobState,
    pub message: Option<String>,
    /// Set when the job waits because no model is set up for this role yet.
    #[serde(default)]
    pub waiting_for: Option<crate::providers::Role>,
}

impl Session {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let lib = Library::open(root)?;
        let device = lib.device()?;
        let queue = Queue::open(&lib.db_path())?;
        queue.recover()?;
        if let Some(op) = crate::changeset::recover(&lib)? {
            tracing::warn!("rolled back half-applied op {op} after an interrupted run");
        }
        Ok(Self {
            lib,
            device,
            queue: Mutex::new(queue),
            sync_lock: Mutex::new(()),
            index: Mutex::new(None),
            index_dirty: AtomicBool::new(true),
        })
    }

    pub fn library(&self) -> &Library {
        &self.lib
    }

    pub fn device(&self) -> &LocalDevice {
        &self.device
    }

    pub fn queue(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn capture_text(
        &self,
        text: &str,
        vault_hint: Option<String>,
        now: &Zoned,
    ) -> Result<RawItem> {
        let item = raw::create(
            &self.lib,
            &self.device,
            now,
            RawKind::Text,
            NewCapture {
                text: text.to_owned(),
                vault_hint,
                assets: vec![],
            },
        )?;
        self.queue()
            .enqueue(JobKind::Ingest, Some(item.id()), json!({}), true)?;
        Ok(item)
    }

    /// A finished voice conversation's transcript (§8.4), filed like any capture.
    pub fn capture_conversation(&self, text: &str, now: &Zoned) -> Result<RawItem> {
        let item = raw::create(
            &self.lib,
            &self.device,
            now,
            RawKind::VoiceConversation,
            NewCapture {
                text: text.to_owned(),
                vault_hint: None,
                assets: vec![],
            },
        )?;
        self.queue()
            .enqueue(JobKind::Ingest, Some(item.id()), json!({}), true)?;
        Ok(item)
    }

    /// Normalises the image, stores it as an asset and queues the vision description.
    pub fn capture_photo(
        &self,
        bytes: &[u8],
        note: Option<String>,
        vault_hint: Option<String>,
        now: &Zoned,
    ) -> Result<RawItem> {
        let asset = assets::import_image(&self.lib, bytes, now)?;
        let item = raw::create(
            &self.lib,
            &self.device,
            now,
            RawKind::Photo,
            NewCapture {
                text: String::new(),
                vault_hint,
                assets: vec![asset],
            },
        )?;
        let q = self.queue();
        q.enqueue(
            JobKind::Describe,
            Some(item.id()),
            json!({ "note": note }),
            true,
        )?;
        q.enqueue(JobKind::Ingest, Some(item.id()), json!({}), true)?;
        Ok(item)
    }

    /// Keeps the recording on this device and queues transcription.
    pub fn capture_voice(
        &self,
        audio: &Path,
        vault_hint: Option<String>,
        now: &Zoned,
    ) -> Result<RawItem> {
        let item = raw::create(
            &self.lib,
            &self.device,
            now,
            RawKind::Voice,
            NewCapture {
                vault_hint,
                ..Default::default()
            },
        )?;
        if let Err(e) = assets::store_audio(&self.lib, item.id(), audio) {
            let _ = std::fs::remove_file(self.lib.path(&item.path));
            return Err(e);
        }
        let q = self.queue();
        q.enqueue(JobKind::Transcribe, Some(item.id()), json!({}), true)?;
        q.enqueue(JobKind::Ingest, Some(item.id()), json!({}), true)?;
        Ok(item)
    }

    /// Deletes a capture that has not left this device yet (e.g. a recording cancelled after save).
    pub fn discard_unsynced(&self, raw_id: &str) -> Result<bool> {
        let id: ulid::Ulid = raw_id
            .parse()
            .map_err(|_| crate::Error::invalid("bad id"))?;
        let Some(item) = raw::find(&self.lib, id)? else {
            return Ok(false);
        };
        let repo = git2::Repository::open(self.lib.root())?;
        if repo
            .status_file(Path::new(&item.path))?
            .contains(git2::Status::WT_NEW)
        {
            std::fs::remove_file(self.lib.path(&item.path))?;
            if let Some(a) = assets::audio_for(&self.lib, id) {
                let _ = std::fs::remove_file(a);
            }
            let q = self.queue();
            for j in q.jobs_for_raw(id)? {
                q.fail(&j.id, "discarded", false, 0)?;
            }
            return Ok(true);
        }
        Ok(false)
    }

    pub fn day(&self, date: Date) -> Result<Vec<CaptureView>> {
        let items = raw::list_day(&self.lib, date)?;
        let ops = if items.iter().any(|i| i.meta.status == RawStatus::Ingested) {
            crate::ledger::since(&self.lib, date.year, date.month)?
        } else {
            vec![]
        };
        let live = crate::ledger::live_ingests_by_source(&ops);
        let pending_review = if live.is_empty() {
            vec![]
        } else {
            crate::review::list(&self.lib)?
        };
        let filing_of = |raw_id: &str| -> Option<Filing> {
            // The newest live op wins (a replay or re-run supersedes older ones).
            let op_id = live.get(raw_id)?.last()?;
            let e = ops.iter().find(|e| &e.op_id == op_id)?;
            let mine = pending_review
                .iter()
                .filter(|r| r.op_id.as_deref() == Some(op_id));
            Some(Filing {
                op_id: op_id.clone(),
                vaults: vaults_of(e),
                pages_created: e.pages_created.len(),
                pages_updated: e.pages_updated.len(),
                claims_to_review: mine
                    .clone()
                    .filter(|r| r.kind == crate::review::ReviewKind::Claim)
                    .count(),
                to_review: mine.count(),
            })
        };
        let q = self.queue();
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            let jobs = q.jobs_for_raw(item.id())?;
            let failed = jobs.iter().find(|j| {
                j.state == JobState::Failed && j.last_error.as_deref() != Some("discarded")
            });
            let (stage, problem) = match item.meta.status {
                RawStatus::Ingested => (CaptureStage::Filed, None),
                RawStatus::Excluded => (CaptureStage::Excluded, None),
                RawStatus::Pending if failed.is_some() => (
                    CaptureStage::Failed,
                    failed.and_then(|j| j.last_error.clone()),
                ),
                RawStatus::Pending if jobs.iter().any(|j| j.state == JobState::Running) => {
                    (CaptureStage::Working, None)
                }
                RawStatus::Pending => (CaptureStage::Saved, None),
            };
            out.push(CaptureView {
                id: item.meta.id.clone(),
                path: item.path.clone(),
                kind: item.meta.kind,
                captured_at: item.meta.captured_at.clone(),
                device: item.meta.device.clone(),
                text: item.body.clone(),
                vault_hint: item.meta.vault_hint.clone(),
                images: item
                    .meta
                    .assets
                    .iter()
                    .filter(|a| {
                        a.ends_with(".jpg")
                            || a.ends_with(".jpeg")
                            || a.ends_with(".png")
                            || a.ends_with(".webp")
                    })
                    .map(|a| self.lib.path(a).to_string_lossy().into_owned())
                    .collect(),
                stage,
                problem,
                filing: if item.meta.status == RawStatus::Ingested {
                    filing_of(&item.meta.id)
                } else {
                    None
                },
            });
        }
        Ok(out)
    }

    /// Runs queued AI jobs in order until the queue is empty, the network is needed but absent, or
    /// a model role is not configured yet. Returns one report per job touched.
    pub async fn run_jobs(
        &self,
        rt: &AiRuntime,
        online: bool,
        cancel: &Cancel,
    ) -> Result<Vec<JobReport>> {
        let mut reports = Vec::new();
        loop {
            if cancel.is_cancelled() {
                break;
            }
            let job = { self.queue().claim(crate::time::now_ms(), online)? };
            let Some(job) = job else { break };
            let now = jiff::Zoned::now();
            let raw_id: Option<ulid::Ulid> = job.raw_id.as_deref().and_then(|r| r.parse().ok());
            let result: std::result::Result<Option<String>, OpError> = async {
                let item = match raw_id {
                    Some(id) => raw::find(&self.lib, id)?,
                    None => None,
                };
                match (job.kind, item) {
                    (_, None) if raw_id.is_some() => Ok(Some("capture was deleted".into())),
                    (JobKind::Transcribe, Some(item)) => {
                        ops::transcribe(&self.lib, rt, &item).await.map(|_| None)
                    }
                    (JobKind::Describe, Some(item)) => {
                        let note = job
                            .payload
                            .get("note")
                            .and_then(|v| v.as_str())
                            .map(str::to_owned);
                        ops::describe(&self.lib, rt, &item, note.as_deref(), &now)
                            .await
                            .map(|_| None)
                    }
                    (JobKind::Ingest, Some(item)) => {
                        let opts: IngestOptions =
                            serde_json::from_value(job.payload.clone()).unwrap_or_default();
                        match ops::ingest(
                            &self.lib,
                            &self.device,
                            rt,
                            item.id(),
                            &opts,
                            &now,
                            cancel,
                            &self.sync_lock,
                        )
                        .await?
                        {
                            IngestOutcome::Filed(r) => Ok(Some(r.summary)),
                            IngestOutcome::Skipped(why) => Ok(Some(format!("skipped: {why}"))),
                        }
                    }
                    (JobKind::Compensate, _) => {
                        let op_id = job.payload["op_id"].as_str().unwrap_or_default().to_owned();
                        let after: AfterUndo = serde_json::from_value(job.payload["after"].clone())
                            .unwrap_or(AfterUndo::Exclude);
                        crate::audit::compensate(
                            &self.lib,
                            &self.device,
                            rt,
                            &op_id,
                            after,
                            &now,
                            cancel,
                            &self.sync_lock,
                        )
                        .await
                        .map(|_| Some("undone".to_owned()))
                    }
                    (kind, _) => Err(OpError::Permanent(format!(
                        "{} jobs are not supported yet",
                        kind.as_str()
                    ))),
                }
            }
            .await;
            let report = match result {
                Ok(summary) => {
                    self.queue().complete(&job.id)?;
                    JobReport {
                        job_id: job.id,
                        kind: job.kind,
                        raw_id: job.raw_id,
                        state: JobState::Done,
                        message: summary,
                        waiting_for: None,
                    }
                }
                Err(OpError::Provider(p)) if p.kind == ProviderErrorKind::NotConfigured => {
                    // Not a failure: wait until the user sets up a model.
                    self.queue().defer(&job.id)?;
                    reports.push(JobReport {
                        job_id: job.id,
                        kind: job.kind,
                        raw_id: job.raw_id,
                        state: JobState::Queued,
                        message: Some(p.message),
                        waiting_for: Some(match job.kind {
                            JobKind::Transcribe => Role::Stt,
                            JobKind::Describe => Role::Vision,
                            JobKind::Compensate => Role::Ingest,
                            _ if rt.has(Role::Router) => Role::Ingest,
                            _ => Role::Router,
                        }),
                    });
                    break;
                }
                Err(e) => {
                    let transient = e.transient();
                    let msg = e.to_string();
                    let state =
                        self.queue()
                            .fail(&job.id, &msg, transient, crate::time::now_ms())?;
                    let stop = state == JobState::Queued;
                    reports.push(JobReport {
                        job_id: job.id,
                        kind: job.kind,
                        raw_id: job.raw_id,
                        state,
                        message: Some(msg),
                        waiting_for: None,
                    });
                    if stop {
                        break; // FIFO: later jobs wait for this one's retry
                    }
                    continue;
                }
            };
            reports.push(report);
            self.invalidate_index();
        }
        Ok(reports)
    }

    // ─────────────── ask (§4.3) ───────────────

    #[allow(clippy::too_many_arguments)]
    pub async fn ask(
        &self,
        rt: &AiRuntime,
        history: &[crate::ask::Turn],
        question: &str,
        image: Option<(String, Vec<u8>)>,
        scope: &crate::ask::AskScope,
        cancel: &Cancel,
        on_delta: crate::providers::OnDelta<'_>,
    ) -> std::result::Result<crate::ask::Answer, OpError> {
        // Answers read the search cache; make sure it has what was filed or synced last.
        let _ = self.with_index(|_| Ok(()));
        crate::ask::ask(
            &self.lib,
            &self.device,
            rt,
            history,
            question,
            image,
            scope,
            false,
            &jiff::Zoned::now(),
            cancel,
            on_delta,
        )
        .await
    }

    /// "Save to wiki": the answer becomes a capture and is filed like any other.
    pub fn save_answer(
        &self,
        question: &str,
        answer: &str,
        scope: &crate::ask::AskScope,
    ) -> Result<RawItem> {
        let item = crate::ask::answer_capture(
            &self.lib,
            &self.device,
            &jiff::Zoned::now(),
            question,
            answer,
            scope,
        )?;
        self.queue()
            .enqueue(JobKind::Ingest, Some(item.id()), json!({}), true)?;
        Ok(item)
    }

    pub fn save_draft(&self, story: &str, title: &str, text: &str) -> Result<String> {
        let _g = self.sync_lock.lock().unwrap_or_else(|p| p.into_inner());
        let p = crate::ask::save_draft(
            &self.lib,
            &self.device,
            &jiff::Zoned::now(),
            story,
            title,
            text,
        )?;
        self.invalidate_index();
        Ok(p)
    }

    // ─────────────── audit and review (§7, §8.5) ───────────────

    pub fn activity(&self, limit: usize, before: Option<&str>) -> Result<Vec<OpSummary>> {
        crate::audit::activity(&self.lib, limit, before)
    }

    pub fn op(&self, op_id: &str) -> Result<OpSummary> {
        crate::audit::op(&self.lib, op_id)
    }

    pub fn op_diff(&self, op_id: &str) -> Result<Vec<crate::audit::FileDiff>> {
        crate::audit::op_diff(&self.lib, op_id)
    }

    /// Undo (§7): `git revert` now if possible, otherwise a compensating op is queued. With
    /// `refile`, the capture is filed again afterwards (move to vault, re-run with note).
    pub fn undo(&self, op_id: &str, refile: Option<IngestOptions>) -> Result<UndoState> {
        let now = jiff::Zoned::now();
        let after = if refile.is_some() {
            AfterUndo::Refile
        } else {
            AfterUndo::Exclude
        };
        let target = crate::audit::op(&self.lib, op_id)?;
        let outcome = {
            let _g = self.sync_lock.lock().unwrap_or_else(|p| p.into_inner());
            // Files on disk must match HEAD before a revert writes over them.
            sync::commit_local(&self.lib, &self.queue(), &self.device)?;
            crate::audit::try_revert(&self.lib, &self.device, &now, op_id, after)?
        };
        self.invalidate_index();
        let raw_id: Option<ulid::Ulid> = target.sources.first().and_then(|s| s.parse().ok());
        let q = self.queue();
        let state = match outcome {
            UndoOutcome::Reverted(id) => UndoState::Done(id),
            UndoOutcome::NeedsCompensation => {
                q.enqueue(
                    JobKind::Compensate,
                    raw_id,
                    json!({ "op_id": op_id, "after": after }),
                    true,
                )?;
                UndoState::Queued
            }
        };
        if let (Some(opts), Some(id)) = (refile, raw_id) {
            q.enqueue(JobKind::Ingest, Some(id), serde_json::to_value(opts)?, true)?;
        }
        Ok(state)
    }

    /// Files an excluded capture again.
    pub fn include(&self, raw_id: &str) -> Result<()> {
        let id: ulid::Ulid = raw_id
            .parse()
            .map_err(|_| crate::Error::invalid("bad id"))?;
        let item =
            raw::find(&self.lib, id)?.ok_or_else(|| crate::Error::invalid("no such capture"))?;
        if item.meta.status != RawStatus::Excluded {
            return Ok(());
        }
        raw::set_status(&self.lib, &item.path, RawStatus::Pending)?;
        self.queue()
            .enqueue(JobKind::Ingest, Some(id), json!({}), true)?;
        Ok(())
    }

    pub fn review_cards(&self) -> Result<Vec<crate::review_ops::ReviewCard>> {
        crate::review_ops::cards(&self.lib)
    }

    pub fn resolve_review(
        &self,
        item_id: &str,
        resolution: crate::review_ops::Resolution,
    ) -> Result<String> {
        let _g = self.sync_lock.lock().unwrap_or_else(|p| p.into_inner());
        let id = crate::review_ops::resolve(
            &self.lib,
            &self.device,
            &jiff::Zoned::now(),
            item_id,
            resolution,
        )?;
        self.invalidate_index();
        Ok(id)
    }

    // ─────────────── wiki (§8.3, §4.5) ───────────────

    /// Marks the search cache stale; the next query re-reads changed files.
    pub fn invalidate_index(&self) {
        self.index_dirty.store(true, Ordering::SeqCst);
    }

    fn with_index<T>(&self, f: impl FnOnce(&SearchIndex) -> Result<T>) -> Result<T> {
        let mut guard = self.index.lock().unwrap_or_else(|p| p.into_inner());
        if guard.is_none() {
            *guard = Some(SearchIndex::open(&self.lib)?);
        }
        let idx = guard.as_mut().expect("opened above");
        if self.index_dirty.swap(false, Ordering::SeqCst) {
            idx.refresh(&self.lib)?;
        }
        f(idx)
    }

    /// Re-reads files changed outside the app (e.g. Obsidian). Returns pages re-indexed.
    pub fn refresh_index(&self) -> Result<usize> {
        self.index_dirty.store(false, Ordering::SeqCst);
        let mut guard = self.index.lock().unwrap_or_else(|p| p.into_inner());
        if guard.is_none() {
            *guard = Some(SearchIndex::open(&self.lib)?);
        }
        guard.as_mut().expect("opened above").refresh(&self.lib)
    }

    /// Drops and rebuilds the cache (Settings › Repository › Rebuild index).
    pub fn rebuild_index(&self) -> Result<usize> {
        let mut guard = self.index.lock().unwrap_or_else(|p| p.into_inner());
        let mut idx = SearchIndex::open(&self.lib)?;
        let n = idx.rebuild(&self.lib)?;
        *guard = Some(idx);
        self.index_dirty.store(false, Ordering::SeqCst);
        Ok(n)
    }

    pub fn search(&self, query: &str, vaults: &[String], limit: usize) -> Result<Vec<Hit>> {
        self.with_index(|i| i.search(query, vaults, &[], limit))
    }

    pub fn recent_pages(&self, vault: Option<&str>, limit: usize) -> Result<Vec<PageRef>> {
        self.with_index(|i| i.recent(vault, limit))
    }

    pub fn list_dir(&self, dir: &str) -> Result<crate::search::DirListing> {
        self.with_index(|i| i.list_dir(dir))
    }

    pub fn local_graph(&self, path: &str, depth: usize) -> Result<Graph> {
        self.with_index(|i| i.local_graph(path, depth.clamp(1, 2), 60))
    }

    /// Resolves a wikilink target as Obsidian would (for taps in the reader).
    pub fn resolve_link(&self, target: &str) -> Result<Option<String>> {
        let r = crate::pages::resolver(&self.lib)?;
        Ok(r.resolve(target))
    }

    pub fn page(&self, path: &str) -> Result<PageView> {
        if path.contains("..") || !(path.starts_with("vaults/") || path.starts_with("raw/")) {
            return Err(crate::Error::invalid("not a page path"));
        }
        let text = std::fs::read_to_string(self.lib.path(path))?;
        let (meta, body) = match crate::wiki::parse(path, &text) {
            Ok(p) => (p.meta, p.body),
            // Unreadable frontmatter (edited by hand): show the file as it is.
            Err(_) => (
                serde_json::from_value(json!({})).expect("defaults"),
                text.clone(),
            ),
        };
        let backlinks = if path.starts_with("vaults/") {
            self.with_index(|i| i.backlinks(path))?
        } else {
            vec![]
        };
        Ok(PageView {
            path: path.to_owned(),
            hash: crate::wiki::content_hash(&text),
            text,
            body,
            meta,
            backlinks,
        })
    }

    /// Saves an edit from the in-app editor as one human commit (`edit: <page>`).
    pub fn save_page(
        &self,
        path: &str,
        base_hash: &str,
        text: &str,
    ) -> Result<crate::edit::EditOutcome> {
        let _g = self.sync_lock.lock().unwrap_or_else(|p| p.into_inner());
        let out = crate::edit::save_page(&self.lib, &self.device, path, base_hash, text)?;
        self.invalidate_index();
        Ok(out)
    }

    /// Model roles resolved against the shared config, with API keys from secure storage.
    pub fn runtime(&self, secrets: std::collections::HashMap<String, String>) -> Result<AiRuntime> {
        Ok(AiRuntime::new(self.lib.config()?.ai, secrets))
    }

    /// Makes a failed capture's jobs runnable again (the user pressed Retry).
    pub fn retry_capture(&self, raw_id: &str) -> Result<()> {
        let id: ulid::Ulid = raw_id
            .parse()
            .map_err(|_| crate::Error::invalid("bad id"))?;
        let q = self.queue();
        for j in q.jobs_for_raw(id)? {
            if j.state == JobState::Failed && j.last_error.as_deref() != Some("discarded") {
                q.retry(&j.id)?;
            }
        }
        Ok(())
    }

    /// Whether any AI job is waiting to run.
    pub fn has_pending_jobs(&self) -> Result<bool> {
        self.queue().has_pending()
    }

    pub fn status(&self) -> Result<LocalStatus> {
        sync::local_status(&self.lib)
    }

    /// Commits, fetches, integrates and pushes. Concurrent calls are serialised.
    pub fn sync(&self, auth: &GitAuth, now: &Zoned) -> Result<SyncOutcome> {
        let _guard = self.sync_lock.lock().unwrap_or_else(|p| p.into_inner());
        let outcome = {
            let q = self.queue();
            sync::sync(&self.lib, &q, &self.device, auth, now)?
        };
        self.invalidate_index();
        // Dropped AI ops are re-run on the merged state (§5.4 step 3).
        for r in &outcome.replays {
            for s in &r.sources {
                if let Ok(id) = s.parse() {
                    self.queue().enqueue(
                        JobKind::Ingest,
                        Some(id),
                        json!({ "replayed_from": r.op_id, "forced_vault": r.forced_vault, "note": r.note }),
                        true,
                    )?;
                }
            }
        }
        Ok(outcome)
    }
}

/// Vaults an op wrote to, in first-touched order, from the pages it created or updated.
fn vaults_of(e: &crate::ledger::LedgerEntry) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in e.pages_created.iter().chain(&e.pages_updated) {
        if let Some(v) = crate::pages::vault_of(p)
            && !out.iter().any(|x| x == v)
        {
            out.push(v.to_owned());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{lib_in, zoned};

    fn session() -> (tempfile::TempDir, Session) {
        let (d, lib) = lib_in();
        lib.set_device(
            "Pixel 8",
            "android",
            &zoned("2026-09-23T09:00:00+03:30[Asia/Tehran]"),
        )
        .unwrap();
        let s = Session::open(lib.root()).unwrap();
        (d, s)
    }

    #[test]
    fn day_view_reflects_pipeline_state() {
        let (_d, s) = session();
        let now = zoned("2026-09-23T10:00:00+03:30[Asia/Tehran]");
        let t = s
            .capture_text("Sara called", Some("life".into()), &now)
            .unwrap();
        let view = s.day(crate::time::date_of(&now)).unwrap();
        assert_eq!(view.len(), 1);
        assert_eq!(view[0].stage, CaptureStage::Saved);
        assert_eq!(view[0].vault_hint.as_deref(), Some("life"));

        raw::set_status(s.library(), &t.path, RawStatus::Ingested).unwrap();
        assert_eq!(
            s.day(crate::time::date_of(&now)).unwrap()[0].stage,
            CaptureStage::Filed
        );
    }

    #[test]
    fn filed_captures_carry_what_filing_did() {
        use crate::ledger::{LedgerEntry, OpType};
        use crate::review::{ReviewItem, ReviewKind};

        let (_d, s) = session();
        let now = zoned("2026-09-23T10:00:00+03:30[Asia/Tehran]");
        let t = s.capture_text("سر درد دارم", None, &now).unwrap();
        let op = crate::ids::new_id().to_string();
        crate::ledger::write(
            s.library(),
            &LedgerEntry {
                op_id: op.clone(),
                op_type: OpType::Ingest,
                sources: vec![t.meta.id.clone()],
                router: None,
                models: vec![],
                pages_created: vec!["vaults/life/journal/2026/2026-09-23.md".into()],
                pages_updated: vec![
                    "vaults/life/people/sara.md".into(),
                    "vaults/health/profile.md".into(),
                    "vaults/health/symptoms-log.md".into(),
                ],
                claims_added: vec!["c-1".into()],
                review_items: vec![],
                usage: Default::default(),
                started_at: String::new(),
                finished_at: String::new(),
                device: "pixel-8".into(),
                summary: String::new(),
                note: None,
                forced_vault: None,
                replayed_from: None,
                reverts: None,
                rejected_claims: vec![],
            },
        )
        .unwrap();
        for kind in [ReviewKind::Claim, ReviewKind::Routing] {
            crate::review::add(
                s.library(),
                &ReviewItem::new(kind, "pixel-8", &now, Some(op.clone()), json!({})),
            )
            .unwrap();
        }
        raw::set_status(s.library(), &t.path, RawStatus::Ingested).unwrap();

        let f = s.day(crate::time::date_of(&now)).unwrap()[0]
            .filing
            .clone()
            .expect("filed capture has a filing");
        assert_eq!(f.op_id, op);
        assert_eq!(f.vaults, vec!["life".to_string(), "health".to_string()]);
        assert_eq!((f.pages_created, f.pages_updated), (1, 3));
        assert_eq!((f.claims_to_review, f.to_review), (1, 2));
    }

    #[test]
    fn voice_capture_keeps_audio_locally_and_can_be_discarded() {
        let (d, s) = session();
        let audio = d.path().join("rec.m4a");
        std::fs::write(&audio, b"fake audio").unwrap();
        let now = zoned("2026-09-23T10:00:00+03:30[Asia/Tehran]");
        let v = s.capture_voice(&audio, None, &now).unwrap();
        assert!(assets::audio_for(s.library(), v.id()).is_some());
        assert_eq!(s.queue().jobs_for_raw(v.id()).unwrap().len(), 2);
        assert!(s.discard_unsynced(&v.meta.id).unwrap());
        assert!(s.day(crate::time::date_of(&now)).unwrap().is_empty());
    }
}
