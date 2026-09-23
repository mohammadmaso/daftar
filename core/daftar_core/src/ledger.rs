//! Op ledger (§7): one immutable JSON file per AI operation under `.daftar/ledger/<yyyy>/<mm>/`.
//!
//! Files are never edited after creation, so relationships are recorded on the *newer* op:
//! a revert names the op it reverts (`reverts`), a replay names its predecessor (`replayed_from`).
//! "Reverted by" is derived. The commit SHA is not stored in the file (it cannot be known before
//! the commit that contains the file); it is found through the `Op-Id` commit trailer.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;

use serde::{Deserialize, Serialize};

use crate::fsutil::atomic_write;
use crate::library::Library;
use crate::{Result, layout};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OpType {
    Ingest,
    RevertOp,
    Compensate,
    Lint,
    Reflect,
    SaveAnswer,
}

impl OpType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ingest => "ingest",
            Self::RevertOp => "revert-op",
            Self::Compensate => "compensate",
            Self::Lint => "lint",
            Self::Reflect => "reflect",
            Self::SaveAnswer => "save-answer",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cached_input_tokens: u64,
    /// Approximate cost in USD from user-editable prices; `None` when prices are unknown.
    #[serde(default)]
    pub cost_usd: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub op_id: String,
    pub op_type: OpType,
    /// Raw capture ids the op was derived from.
    #[serde(default)]
    pub sources: Vec<String>,
    /// Router decision (targets, reasons, confidences) for ingest ops.
    #[serde(default)]
    pub router: Option<serde_json::Value>,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub pages_created: Vec<String>,
    #[serde(default)]
    pub pages_updated: Vec<String>,
    #[serde(default)]
    pub claims_added: Vec<String>,
    #[serde(default)]
    pub review_items: Vec<String>,
    #[serde(default)]
    pub usage: Usage,
    pub started_at: String,
    pub finished_at: String,
    pub device: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub forced_vault: Option<String>,
    #[serde(default)]
    pub replayed_from: Option<String>,
    #[serde(default)]
    pub reverts: Option<String>,
}

impl LedgerEntry {
    pub fn rel_path(&self) -> String {
        let id: ulid::Ulid = self.op_id.parse().expect("op ids are ULIDs");
        let ts = jiff::Timestamp::from_millisecond(id.timestamp_ms() as i64)
            .expect("ulid timestamp in range")
            .to_zoned(jiff::tz::TimeZone::UTC);
        layout::ledger_entry(crate::time::date_of(&ts), id)
    }
}

pub fn write(lib: &Library, entry: &LedgerEntry) -> Result<String> {
    let rel = entry.rel_path();
    let mut json = serde_json::to_string_pretty(entry)?;
    json.push('\n');
    atomic_write(&lib.path(&rel), json.as_bytes())?;
    Ok(rel)
}

/// All ledger entries, ordered by op id (= chronological).
pub fn all(lib: &Library) -> Result<Vec<LedgerEntry>> {
    let mut out = BTreeMap::new();
    let root = lib.path(".daftar/ledger");
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "json") {
                match fs::read(&p)
                    .map_err(crate::Error::from)
                    .and_then(|b| serde_json::from_slice::<LedgerEntry>(&b).map_err(Into::into))
                {
                    Ok(entry) => {
                        out.insert(entry.op_id.clone(), entry);
                    }
                    Err(err) => tracing::warn!("unreadable ledger entry {}: {err}", p.display()),
                }
            }
        }
    }
    Ok(out.into_values().collect())
}

/// Op ids that have been undone (directly or through an undo of an undo, which re-activates).
pub fn reverted_ops(entries: &[LedgerEntry]) -> HashSet<String> {
    // Reverts always target older ops, so resolving newest-first means every revert's own
    // state is final by the time it is applied.
    let mut reverted: HashSet<String> = HashSet::new();
    for e in entries.iter().rev() {
        if reverted.contains(&e.op_id) {
            continue;
        }
        if let (Some(target), OpType::RevertOp | OpType::Compensate) = (&e.reverts, e.op_type) {
            reverted.insert(target.clone());
        }
    }
    reverted
}

/// Live ingest ops per raw source id.
pub fn live_ingests_by_source(entries: &[LedgerEntry]) -> HashMap<String, Vec<String>> {
    let reverted = reverted_ops(entries);
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for e in entries {
        if e.op_type == OpType::Ingest && !reverted.contains(&e.op_id) {
            for s in &e.sources {
                map.entry(s.clone()).or_default().push(e.op_id.clone());
            }
        }
    }
    for ops in map.values_mut() {
        ops.sort();
    }
    map
}

/// Commit message with trailers (§5.5).
pub fn commit_message(
    subject: &str,
    entry: &LedgerEntry,
    source_path: Option<&str>,
    vaults: &[String],
) -> String {
    let mut msg = format!(
        "{subject}\n\nOp-Id: {}\nOp-Type: {}\n",
        entry.op_id,
        entry.op_type.as_str()
    );
    if let Some(p) = source_path {
        msg.push_str(&format!("Source: {p}\n"));
    }
    if !vaults.is_empty() {
        msg.push_str(&format!("Vaults: {}\n", vaults.join(", ")));
    }
    if let Some(r) = &entry.replayed_from {
        msg.push_str(&format!("Replayed-From: {r}\n"));
    }
    if let Some(r) = &entry.reverts {
        msg.push_str(&format!("Reverts: {r}\n"));
    }
    msg.push_str(&format!("Device: {}\n", entry.device));
    msg
}

/// Extracts a trailer value from a commit message.
pub fn trailer<'a>(message: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}: ");
    message
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix(prefix.as_str()))
        .map(str::trim)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, t: OpType, src: &[&str], reverts: Option<&str>) -> LedgerEntry {
        LedgerEntry {
            op_id: id.into(),
            op_type: t,
            sources: src.iter().map(|s| s.to_string()).collect(),
            router: None,
            models: vec![],
            pages_created: vec![],
            pages_updated: vec![],
            claims_added: vec![],
            review_items: vec![],
            usage: Usage::default(),
            started_at: String::new(),
            finished_at: String::new(),
            device: "d".into(),
            summary: String::new(),
            note: None,
            forced_vault: None,
            replayed_from: None,
            reverts: reverts.map(Into::into),
        }
    }

    #[test]
    fn undo_of_undo_reactivates() {
        let a = "01J00000000000000000000001";
        let r1 = "01J00000000000000000000002";
        let r2 = "01J00000000000000000000003";
        let mut es = vec![entry(a, OpType::Ingest, &["raw1"], None)];
        es.push(entry(r1, OpType::RevertOp, &[], Some(a)));
        assert!(reverted_ops(&es).contains(a));
        assert!(!live_ingests_by_source(&es).contains_key("raw1"));
        es.push(entry(r2, OpType::RevertOp, &[], Some(r1)));
        let rev = reverted_ops(&es);
        assert!(rev.contains(r1) && !rev.contains(a));
        assert_eq!(live_ingests_by_source(&es)["raw1"], vec![a.to_string()]);
    }

    #[test]
    fn trailers() {
        let e = entry("01J00000000000000000000001", OpType::Ingest, &[], None);
        let m = commit_message(
            "ingest: voice note → life",
            &e,
            Some("raw/x.md"),
            &["life".into()],
        );
        assert_eq!(trailer(&m, "Op-Id"), Some("01J00000000000000000000001"));
        assert_eq!(trailer(&m, "Vaults"), Some("life"));
    }
}
