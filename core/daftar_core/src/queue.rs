//! Durable job queue (§4) in the device-local SQLite cache.
//!
//! Jobs survive restarts, retry with exponential backoff, and wait for the network when they need
//! it. Jobs in the `ai` lane run strictly in creation order so captures are ingested in the order
//! they were made (scenario 1); a job that is retrying blocks later ones, a job that failed for
//! good does not.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    Transcribe,
    Describe,
    Ingest,
    Query,
    Lint,
    Reflect,
    Compensate,
}

impl JobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transcribe => "transcribe",
            Self::Describe => "describe",
            Self::Ingest => "ingest",
            Self::Query => "query",
            Self::Lint => "lint",
            Self::Reflect => "reflect",
            Self::Compensate => "compensate",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "transcribe" => Self::Transcribe,
            "describe" => Self::Describe,
            "ingest" => Self::Ingest,
            "query" => Self::Query,
            "lint" => Self::Lint,
            "reflect" => Self::Reflect,
            "compensate" => Self::Compensate,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Done,
    Failed,
}

impl JobState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "running" => Self::Running,
            "done" => Self::Done,
            "failed" => Self::Failed,
            _ => Self::Queued,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub kind: JobKind,
    /// Raw capture this job is about, if any.
    pub raw_id: Option<String>,
    pub payload: serde_json::Value,
    pub state: JobState,
    pub attempts: u32,
    pub run_after_ms: i64,
    pub last_error: Option<String>,
    pub needs_network: bool,
}

pub const MAX_ATTEMPTS: u32 = 8;
const BASE_BACKOFF_MS: i64 = 5_000;
const MAX_BACKOFF_MS: i64 = 30 * 60_000;

pub struct Queue {
    conn: Connection,
}

impl Queue {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    pub fn in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA busy_timeout = 5000;
             CREATE TABLE IF NOT EXISTS jobs (
               id TEXT PRIMARY KEY,
               kind TEXT NOT NULL,
               raw_id TEXT,
               payload TEXT NOT NULL DEFAULT '{}',
               state TEXT NOT NULL,
               attempts INTEGER NOT NULL DEFAULT 0,
               run_after_ms INTEGER NOT NULL DEFAULT 0,
               last_error TEXT,
               needs_network INTEGER NOT NULL DEFAULT 1,
               lane TEXT NOT NULL DEFAULT 'ai',
               updated_ms INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS jobs_state ON jobs(state);
             CREATE INDEX IF NOT EXISTS jobs_raw ON jobs(raw_id);",
        )?;
        Ok(Self { conn })
    }

    pub fn enqueue(
        &self,
        kind: JobKind,
        raw_id: Option<Ulid>,
        payload: serde_json::Value,
        needs_network: bool,
    ) -> Result<Job> {
        let id = Ulid::generate().to_string();
        let now = crate::time::now_ms();
        self.conn.execute(
            "INSERT INTO jobs (id, kind, raw_id, payload, state, needs_network, updated_ms)
             VALUES (?1, ?2, ?3, ?4, 'queued', ?5, ?6)",
            params![
                id,
                kind.as_str(),
                raw_id.map(|r| r.to_string()),
                payload.to_string(),
                needs_network,
                now
            ],
        )?;
        Ok(self.get(&id)?.expect("just inserted"))
    }

    pub fn get(&self, id: &str) -> Result<Option<Job>> {
        Ok(self
            .conn
            .query_row(&format!("{SELECT} WHERE id = ?1"), [id], row_to_job)
            .optional()?)
    }

    /// After a crash, jobs left `running` are re-queued; job handlers must be idempotent.
    pub fn recover(&self) -> Result<usize> {
        Ok(self.conn.execute(
            "UPDATE jobs SET state = 'queued' WHERE state = 'running'",
            [],
        )?)
    }

    /// Claims the next runnable job, honouring strict FIFO in the `ai` lane.
    pub fn claim(&self, now_ms: i64, online: bool) -> Result<Option<Job>> {
        let head: Option<Job> = self
            .conn
            .query_row(
                &format!("{SELECT} WHERE state IN ('queued','running') ORDER BY rowid LIMIT 1"),
                [],
                row_to_job,
            )
            .optional()?;
        let Some(job) = head else { return Ok(None) };
        let runnable = job.state == JobState::Queued
            && job.run_after_ms <= now_ms
            && (online || !job.needs_network);
        if !runnable {
            return Ok(None);
        }
        self.conn.execute(
            "UPDATE jobs SET state = 'running', attempts = attempts + 1, updated_ms = ?2 WHERE id = ?1",
            params![job.id, now_ms],
        )?;
        self.get(&job.id)
    }

    pub fn complete(&self, id: &str) -> Result<()> {
        self.set_state(id, JobState::Done, None, 0)
    }

    /// Records a failure. Transient failures retry with backoff until `MAX_ATTEMPTS`.
    pub fn fail(&self, id: &str, error: &str, transient: bool, now_ms: i64) -> Result<JobState> {
        let job = self
            .get(id)?
            .ok_or_else(|| crate::Error::invalid("unknown job"))?;
        if transient && job.attempts < MAX_ATTEMPTS {
            let backoff = (BASE_BACKOFF_MS << job.attempts.min(16)).min(MAX_BACKOFF_MS);
            self.set_state(id, JobState::Queued, Some(error), now_ms + backoff)?;
            Ok(JobState::Queued)
        } else {
            self.set_state(id, JobState::Failed, Some(error), 0)?;
            Ok(JobState::Failed)
        }
    }

    /// Puts a job back without counting an attempt (e.g. network dropped before it started).
    pub fn defer(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE jobs SET state = 'queued', attempts = max(attempts - 1, 0) WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    /// Makes a failed job runnable again (user pressed "Retry").
    pub fn retry(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE jobs SET state = 'queued', attempts = 0, run_after_ms = 0, last_error = NULL WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    fn set_state(&self, id: &str, s: JobState, err: Option<&str>, run_after: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE jobs SET state = ?2, last_error = ?3, run_after_ms = ?4, updated_ms = ?5 WHERE id = ?1",
            params![id, s.as_str(), err, run_after, crate::time::now_ms()],
        )?;
        Ok(())
    }

    /// Whether any job is waiting or running (failed jobs wait for the user and do not count).
    pub fn has_pending(&self) -> Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM jobs WHERE state IN ('queued','running')",
            [],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Jobs not yet done, oldest first.
    pub fn open_jobs(&self) -> Result<Vec<Job>> {
        let mut stmt = self
            .conn
            .prepare(&format!("{SELECT} WHERE state != 'done' ORDER BY rowid"))?;
        let rows = stmt.query_map([], row_to_job)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn jobs_for_raw(&self, raw_id: Ulid) -> Result<Vec<Job>> {
        let mut stmt = self
            .conn
            .prepare(&format!("{SELECT} WHERE raw_id = ?1 ORDER BY rowid"))?;
        let rows = stmt.query_map([raw_id.to_string()], row_to_job)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Whether any not-yet-done job still has to produce this capture's body.
    pub fn awaiting_body(&self, raw_id: &str) -> Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM jobs WHERE raw_id = ?1 AND kind IN ('transcribe','describe') AND state != 'done'",
            [raw_id],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }
}

const SELECT: &str = "SELECT id, kind, raw_id, payload, state, attempts, run_after_ms, last_error, needs_network FROM jobs";

fn row_to_job(r: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
    let kind: String = r.get(1)?;
    let payload: String = r.get(3)?;
    let state: String = r.get(4)?;
    Ok(Job {
        id: r.get(0)?,
        kind: JobKind::parse(&kind).unwrap_or(JobKind::Ingest),
        raw_id: r.get(2)?,
        payload: serde_json::from_str(&payload).unwrap_or_default(),
        state: JobState::parse(&state),
        attempts: r.get(5)?,
        run_after_ms: r.get(6)?,
        last_error: r.get(7)?,
        needs_network: r.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fifo_waits_for_network_and_backoff() {
        let q = Queue::in_memory().unwrap();
        let a = q
            .enqueue(JobKind::Ingest, None, json!({"n": 1}), true)
            .unwrap();
        let b = q
            .enqueue(JobKind::Ingest, None, json!({"n": 2}), true)
            .unwrap();

        assert!(
            q.claim(0, false).unwrap().is_none(),
            "offline: nothing runs"
        );

        let got = q.claim(0, true).unwrap().unwrap();
        assert_eq!(got.id, a.id);
        assert!(
            q.claim(0, true).unwrap().is_none(),
            "head is running, b must wait"
        );

        assert_eq!(
            q.fail(&a.id, "timeout", true, 1_000).unwrap(),
            JobState::Queued
        );
        assert!(
            q.claim(1_000, true).unwrap().is_none(),
            "a is backing off and blocks b"
        );
        let retried = q.claim(1_000 + 60_000, true).unwrap().unwrap();
        assert_eq!(retried.id, a.id);
        assert_eq!(retried.attempts, 2);
        q.complete(&a.id).unwrap();

        assert_eq!(q.claim(0, true).unwrap().unwrap().id, b.id);
    }

    #[test]
    fn permanent_failure_unblocks_the_lane_and_recover_requeues() {
        let q = Queue::in_memory().unwrap();
        let a = q
            .enqueue(JobKind::Transcribe, None, json!({}), true)
            .unwrap();
        let b = q.enqueue(JobKind::Ingest, None, json!({}), true).unwrap();
        q.claim(0, true).unwrap();
        assert_eq!(
            q.fail(&a.id, "bad audio", false, 0).unwrap(),
            JobState::Failed
        );
        assert_eq!(q.claim(0, true).unwrap().unwrap().id, b.id);
        assert_eq!(q.recover().unwrap(), 1);
        assert_eq!(q.get(&b.id).unwrap().unwrap().state, JobState::Queued);
    }

    #[test]
    fn awaiting_body_tracks_transcription() {
        let q = Queue::in_memory().unwrap();
        let raw = Ulid::generate();
        let t = q
            .enqueue(JobKind::Transcribe, Some(raw), json!({}), true)
            .unwrap();
        assert!(q.awaiting_body(&raw.to_string()).unwrap());
        q.complete(&t.id).unwrap();
        assert!(!q.awaiting_body(&raw.to_string()).unwrap());
    }
}
