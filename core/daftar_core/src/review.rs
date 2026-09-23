//! Review queue items (§8.5), stored as one JSON file each under `.daftar/review/` so that items
//! created on different devices never conflict and every device sees the same queue.
//! Resolving an item deletes its file (with the resolution recorded in the op that acts on it).

use std::fs;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::Result;
use crate::fsutil::atomic_write;
use crate::library::Library;

pub const DIR: &str = ".daftar/review";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewKind {
    Claim,
    Routing,
    Lint,
    SyncConflict,
    Schema,
    Question,
    HumanEdit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewItem {
    pub id: String,
    pub kind: ReviewKind,
    pub created_at: String,
    pub device: String,
    /// Op that created the item, if any.
    #[serde(default)]
    pub op_id: Option<String>,
    pub payload: serde_json::Value,
}

impl ReviewItem {
    pub fn new(
        kind: ReviewKind,
        device: &str,
        now: &jiff::Zoned,
        op_id: Option<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Ulid::generate().to_string(),
            kind,
            created_at: crate::time::rfc3339(now),
            device: device.to_owned(),
            op_id,
            payload,
        }
    }

    pub fn rel_path(&self) -> String {
        format!("{DIR}/{}.json", self.id)
    }
}

pub fn add(lib: &Library, item: &ReviewItem) -> Result<String> {
    let rel = item.rel_path();
    let mut json = serde_json::to_string_pretty(item)?;
    json.push('\n');
    atomic_write(&lib.path(&rel), json.as_bytes())?;
    Ok(rel)
}

pub fn list(lib: &Library) -> Result<Vec<ReviewItem>> {
    let mut items = Vec::new();
    let Ok(entries) = fs::read_dir(lib.path(DIR)) else {
        return Ok(items);
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "json")
            && let Ok(item) = serde_json::from_slice::<ReviewItem>(&fs::read(&p)?)
        {
            items.push(item);
        }
    }
    items.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(items)
}

pub fn remove(lib: &Library, id: &str) -> Result<()> {
    let p = lib.path(&format!("{DIR}/{id}.json"));
    if p.exists() {
        fs::remove_file(p)?;
    }
    Ok(())
}
