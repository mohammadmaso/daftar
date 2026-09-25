//! Full-text search over the wiki (§4.5). A rebuildable cache in `.git/daftar/search.sqlite`:
//! FTS5 over normalised text (see `normalize`) with a trigram table as fallback for partial words.
//! `refresh` is incremental by (mtime, size), so edits made outside the app are picked up.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::library::Library;
use crate::normalize::{index_form, normalize};
use crate::pages;
use crate::wiki::Page;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub path: String,
    pub vault: String,
    pub kind: String,
    pub title_en: String,
    pub title_fa: String,
    pub summary: String,
    pub snippet: String,
    pub updated: String,
    pub score: f64,
}

pub struct SearchIndex {
    conn: Connection,
}

const SCHEMA_VERSION: i64 = 2;

impl SearchIndex {
    pub fn open(lib: &Library) -> Result<Self> {
        Self::open_at(&lib.local_dir().join("search.sqlite"))
    }

    pub fn open_at(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA busy_timeout = 5000;",
        )?;
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if v != SCHEMA_VERSION {
            conn.execute_batch(
                "DROP TABLE IF EXISTS pages; DROP TABLE IF EXISTS pages_fts; DROP TABLE IF EXISTS pages_tri;
                 CREATE TABLE pages (
                   path TEXT PRIMARY KEY, vault TEXT, kind TEXT, title_en TEXT, title_fa TEXT,
                   aliases TEXT, summary TEXT, updated TEXT, body TEXT, mtime INTEGER, size INTEGER
                 );
                 CREATE VIRTUAL TABLE pages_fts USING fts5(
                   path UNINDEXED, title, aliases, summary, body,
                   tokenize = 'unicode61 remove_diacritics 2'
                 );
                 CREATE VIRTUAL TABLE pages_tri USING fts5(path UNINDEXED, text, tokenize = 'trigram');",
            )?;
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        Ok(Self { conn })
    }

    /// Indexes new/changed pages and drops deleted ones. Returns the number of pages (re)indexed.
    pub fn refresh(&mut self, lib: &Library) -> Result<usize> {
        let mut known: HashMap<String, (i64, i64)> = HashMap::new();
        {
            let mut stmt = self.conn.prepare("SELECT path, mtime, size FROM pages")?;
            for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get(1)?, r.get(2)?)))? {
                let (p, m, s) = row?;
                known.insert(p, (m, s));
            }
        }
        let tx = self.conn.transaction()?;
        let mut n = 0;
        for path in pages::page_paths(lib)? {
            let meta = fs::metadata(lib.path(&path))?;
            let mtime = meta
                .modified()?
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            let size = meta.len() as i64;
            if known.remove(&path) == Some((mtime, size)) {
                continue;
            }
            match pages::read(lib, &path) {
                Ok(page) => {
                    upsert(&tx, &page, mtime, size)?;
                    n += 1;
                }
                Err(e) => tracing::warn!("not indexing {path}: {e}"),
            }
        }
        for gone in known.keys() {
            delete(&tx, gone)?;
        }
        tx.commit()?;
        Ok(n)
    }

    pub fn rebuild(&mut self, lib: &Library) -> Result<usize> {
        self.conn
            .execute_batch("DELETE FROM pages; DELETE FROM pages_fts; DELETE FROM pages_tri;")?;
        self.refresh(lib)
    }

    /// Ranked search. `vaults` / `kinds` filter when non-empty.
    pub fn search(
        &self,
        query: &str,
        vaults: &[String],
        kinds: &[String],
        limit: usize,
    ) -> Result<Vec<Hit>> {
        let q = normalize(query);
        let terms: Vec<String> = q
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
            .map(|t| format!("\"{}\"", t.replace('"', "")))
            .collect();
        if terms.is_empty() {
            return Ok(vec![]);
        }
        // Prefix match on the last term so results appear while typing.
        let mut fts_q = terms.join(" ");
        fts_q.push('*');
        let mut hits = self.query_fts(&fts_q, limit * 3)?;
        if hits.len() < limit && q.chars().count() >= 3 {
            for h in self.query_trigram(&q, limit * 3)? {
                if !hits.iter().any(|x| x.path == h.path) {
                    hits.push(h);
                }
            }
        }
        hits.retain(|h| {
            (vaults.is_empty() || vaults.contains(&h.vault))
                && (kinds.is_empty() || kinds.contains(&h.kind))
        });
        hits.truncate(limit);
        Ok(hits)
    }

    fn query_fts(&self, q: &str, limit: usize) -> Result<Vec<Hit>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.path, p.vault, p.kind, p.title_en, p.title_fa, p.summary, p.updated, p.body,
                    bm25(pages_fts, 0.0, 10.0, 8.0, 4.0, 1.0) AS rank
             FROM pages_fts JOIN pages p ON p.path = pages_fts.path
             WHERE pages_fts MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![q, limit as i64], |r| row_to_hit(r, q))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    fn query_trigram(&self, q: &str, limit: usize) -> Result<Vec<Hit>> {
        let phrase = format!("\"{}\"", q.replace('"', ""));
        let mut stmt = self.conn.prepare(
            "SELECT p.path, p.vault, p.kind, p.title_en, p.title_fa, p.summary, p.updated, p.body, 0.0
             FROM pages_tri JOIN pages p ON p.path = pages_tri.path
             WHERE pages_tri MATCH ?1 LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![phrase, limit as i64], |r| row_to_hit(r, q))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn count(&self) -> Result<usize> {
        Ok(self
            .conn
            .query_row("SELECT count(*) FROM pages", [], |r| r.get::<_, i64>(0))?
            as usize)
    }
}

fn upsert(tx: &rusqlite::Transaction<'_>, p: &Page, mtime: i64, size: i64) -> Result<()> {
    delete(tx, &p.path)?;
    let vault = pages::vault_of(&p.path).unwrap_or_default().to_owned();
    let aliases = p.meta.aliases.join(" | ");
    tx.execute(
        "INSERT INTO pages (path, vault, kind, title_en, title_fa, aliases, summary, updated, body, mtime, size)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![p.path, vault, p.meta.kind, p.meta.title.en, p.meta.title.fa, aliases, p.meta.summary, p.meta.updated, p.body, mtime, size],
    )?;
    let title = index_form(&format!(
        "{} {} {}",
        p.meta.title.en,
        p.meta.title.fa,
        p.slug().replace('-', " ")
    ));
    tx.execute(
        "INSERT INTO pages_fts (path, title, aliases, summary, body) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            p.path,
            title,
            index_form(&aliases),
            index_form(&p.meta.summary),
            index_form(&p.body)
        ],
    )?;
    let all = format!(
        "{title} {} {} {}",
        index_form(&aliases),
        index_form(&p.meta.summary),
        index_form(&p.body)
    );
    tx.execute(
        "INSERT INTO pages_tri (path, text) VALUES (?1, ?2)",
        params![p.path, all],
    )?;
    Ok(())
}

fn delete(tx: &rusqlite::Transaction<'_>, path: &str) -> Result<()> {
    tx.execute("DELETE FROM pages WHERE path = ?1", [path])?;
    tx.execute("DELETE FROM pages_fts WHERE path = ?1", [path])?;
    tx.execute("DELETE FROM pages_tri WHERE path = ?1", [path])?;
    Ok(())
}

fn row_to_hit(r: &rusqlite::Row<'_>, q: &str) -> rusqlite::Result<Hit> {
    let body: String = r.get(7)?;
    Ok(Hit {
        path: r.get(0)?,
        vault: r.get(1)?,
        kind: r.get(2)?,
        title_en: r.get(3)?,
        title_fa: r.get(4)?,
        summary: r.get(5)?,
        updated: r.get(6)?,
        snippet: snippet(&body, q),
        score: -r.get::<_, f64>(8)?,
    })
}

/// A readable excerpt of the original text around the first matching term.
pub fn snippet(body: &str, normalized_query: &str) -> String {
    let terms: Vec<&str> = normalized_query
        .split_whitespace()
        .filter(|t| !t.trim_matches('*').is_empty())
        .collect();
    for line in body.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let n = normalize(l);
        if terms
            .iter()
            .any(|t| n.contains(t.trim_matches(|c| c == '"' || c == '*')))
        {
            return l.chars().take(180).collect();
        }
    }
    body.lines()
        .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .unwrap_or("")
        .chars()
        .take(180)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::lib_in;

    fn write(
        lib: &Library,
        rel: &str,
        title_en: &str,
        title_fa: &str,
        aliases: &[&str],
        body: &str,
    ) {
        let doc = format!(
            "---\ntype: topic\ntitle: {{ en: \"{title_en}\", fa: \"{title_fa}\" }}\naliases: [{}]\nsummary: \"\"\n---\n\n{body}\n",
            aliases
                .iter()
                .map(|a| format!("\"{a}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        crate::fsutil::atomic_write(&lib.path(rel), doc.as_bytes()).unwrap();
    }

    /// Scenario 7: Persian search variants.
    #[test]
    fn persian_variants_find_the_same_pages() {
        let (_d, lib) = lib_in();
        write(
            &lib,
            "vaults/health/conditions/vitamin-d-deficiency.md",
            "Vitamin D deficiency",
            "کمبود ویتامین D",
            &["vitamin d", "ویتامین دی"],
            "سطح ویتامین D در آزمایش ۱۴۰۵ پایین بود. دکتر گفت می‌خواهد دوباره بسنجد.",
        );
        write(
            &lib,
            "vaults/life/people/ali.md",
            "Ali",
            "علی",
            &[],
            "علی کتاب را آورد. Meeting با Ali درباره‌ی project.",
        );
        let mut idx = SearchIndex::open(&lib).unwrap();
        assert_eq!(idx.refresh(&lib).unwrap(), 2);
        let top = |q: &str| {
            idx.search(q, &[], &[], 5)
                .unwrap()
                .first()
                .map(|h| h.path.clone())
                .unwrap_or_default()
        };

        assert_eq!(top("علي"), "vaults/life/people/ali.md", "Arabic yeh");
        assert_eq!(top("كتاب"), "vaults/life/people/ali.md", "Arabic kaf");
        assert!(top("ويتامين").contains("vitamin"), "Arabic yeh in word");
        assert!(top("١٤٠٥").contains("vitamin"), "Arabic-Indic digits");
        assert!(top("1405").contains("vitamin"), "ASCII digits");
        assert!(top("می‌خواهد").contains("vitamin"), "with ZWNJ");
        assert!(top("میخواهد").contains("vitamin"), "without ZWNJ");
        assert!(top("VITAMIN").contains("vitamin"), "case folding");
        assert!(top("meeting ali").contains("ali"), "mixed fa/en");
        assert!(top("درباره‌ی project").contains("ali"), "mixed with ZWNJ");
        assert!(top("ویتا").contains("vitamin"), "prefix while typing");
        assert!(top("تامین").contains("vitamin"), "trigram partial");
    }

    #[test]
    fn refresh_is_incremental_and_handles_deletes() {
        let (_d, lib) = lib_in();
        write(
            &lib,
            "vaults/work/topics/rust.md",
            "Rust",
            "راست",
            &[],
            "ownership",
        );
        let mut idx = SearchIndex::open(&lib).unwrap();
        assert_eq!(idx.refresh(&lib).unwrap(), 1);
        assert_eq!(idx.refresh(&lib).unwrap(), 0, "unchanged files are skipped");
        std::fs::remove_file(lib.path("vaults/work/topics/rust.md")).unwrap();
        idx.refresh(&lib).unwrap();
        assert_eq!(idx.count().unwrap(), 0);
        assert!(idx.search("ownership", &[], &[], 5).unwrap().is_empty());
    }
}
