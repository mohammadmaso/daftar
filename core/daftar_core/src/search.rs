//! Full-text search over the wiki (§4.5). A rebuildable cache in `.git/daftar/search.sqlite`:
//! FTS5 over normalised text (see `normalize`) with a trigram table as fallback for partial words.
//! `refresh` is incremental by (mtime, size), so edits made outside the app are picked up.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use rusqlite::{Connection, OptionalExtension, params};
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

const SCHEMA_VERSION: i64 = 3;

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
                 DROP TABLE IF EXISTS links;
                 CREATE TABLE pages (
                   path TEXT PRIMARY KEY, vault TEXT, kind TEXT, title_en TEXT, title_fa TEXT,
                   aliases TEXT, summary TEXT, updated TEXT, body TEXT, mtime INTEGER, size INTEGER
                 );
                 CREATE VIRTUAL TABLE pages_fts USING fts5(
                   path UNINDEXED, title, aliases, summary, body,
                   tokenize = 'unicode61 remove_diacritics 2'
                 );
                 CREATE VIRTUAL TABLE pages_tri USING fts5(path UNINDEXED, text, tokenize = 'trigram');
                 -- Outgoing wikilinks, unresolved (targets may be created later). `slug` is the
                 -- lower-cased last path segment, the key Obsidian-style resolution starts from.
                 CREATE TABLE links (src TEXT NOT NULL, target TEXT NOT NULL, slug TEXT NOT NULL);
                 CREATE INDEX links_src ON links(src);
                 CREATE INDEX links_slug ON links(slug);
                 CREATE INDEX pages_updated ON pages(vault, updated);",
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
        self.conn.execute_batch(
            "DELETE FROM pages; DELETE FROM pages_fts; DELETE FROM pages_tri; DELETE FROM links;",
        )?;
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

    /// Every indexed page path.
    pub fn paths(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT path FROM pages")?;
        let rows = stmt.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    fn resolver(&self) -> Result<pages::Resolver> {
        Ok(pages::Resolver::new(
            self.paths()?.iter().map(String::as_str),
        ))
    }

    /// Pages linking to `path`, as (source path, title_en, title_fa), sorted by source path.
    pub fn backlinks(&self, path: &str) -> Result<Vec<PageRef>> {
        let resolver = self.resolver()?;
        let slug = crate::wiki::slug_of(path).to_lowercase();
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT src, target FROM links WHERE slug = ?1 AND src != ?2")?;
        let rows = stmt.query_map(params![slug, path], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        let mut srcs: Vec<String> = Vec::new();
        for row in rows {
            let (src, target) = row?;
            if resolver.resolve(&target).as_deref() == Some(path) && !srcs.contains(&src) {
                srcs.push(src);
            }
        }
        srcs.sort();
        srcs.iter().map(|p| self.page_ref(p)).collect()
    }

    /// Pages `path` links to that exist (raw citations and unresolved links are left out).
    pub fn outlinks(&self, path: &str) -> Result<Vec<PageRef>> {
        let resolver = self.resolver()?;
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT target FROM links WHERE src = ?1")?;
        let targets: Vec<String> = stmt
            .query_map([path], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        let mut out: Vec<String> = targets
            .iter()
            .filter_map(|t| resolver.resolve(t))
            .filter(|p| p != path)
            .collect();
        out.sort();
        out.dedup();
        out.iter().map(|p| self.page_ref(p)).collect()
    }

    /// Local graph around `path` (§8.3): pages within `depth` links in either direction, at most
    /// `max_nodes`, nearest first. Edges are directed source → target.
    pub fn local_graph(&self, path: &str, depth: usize, max_nodes: usize) -> Result<Graph> {
        let mut nodes = vec![GraphNode {
            page: self.page_ref(path)?,
            depth: 0,
        }];
        let mut edges: Vec<(String, String)> = Vec::new();
        let mut frontier = vec![path.to_owned()];
        for d in 1..=depth {
            let mut next = Vec::new();
            for p in &frontier {
                let out = self.outlinks(p)?;
                let back = self.backlinks(p)?;
                for (r, outgoing) in out
                    .into_iter()
                    .map(|r| (r, true))
                    .chain(back.into_iter().map(|r| (r, false)))
                {
                    let e = if outgoing {
                        (p.clone(), r.path.clone())
                    } else {
                        (r.path.clone(), p.clone())
                    };
                    let known = nodes.iter().any(|n| n.page.path == r.path);
                    if !known {
                        if nodes.len() >= max_nodes {
                            continue;
                        }
                        next.push(r.path.clone());
                        nodes.push(GraphNode { page: r, depth: d });
                    }
                    if !edges.contains(&e) {
                        edges.push(e);
                    }
                }
            }
            frontier = next;
        }
        Ok(Graph { nodes, edges })
    }

    /// The whole wiki as one graph (the Graph view): every page, optionally in one vault, and each
    /// pair of pages joined by a resolved wikilink once, whichever way it points. Links are
    /// resolved against every page, so a vault filter only drops nodes, never re-targets links.
    pub fn wiki_graph(&self, vault: Option<&str>) -> Result<WikiGraph> {
        let resolver = self.resolver()?;
        let mut stmt = self.conn.prepare(
            "SELECT path, vault, kind, title_en, title_fa, summary, updated FROM pages
             WHERE (?1 IS NULL OR vault = ?1) ORDER BY path",
        )?;
        let pages: Vec<PageRef> = stmt
            .query_map(params![vault], row_to_ref)?
            .collect::<Result<_, _>>()?;
        let at: HashMap<&str, usize> = pages
            .iter()
            .enumerate()
            .map(|(i, p)| (p.path.as_str(), i))
            .collect();
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT src, target FROM links ORDER BY src, target")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut seen = std::collections::HashSet::new();
        let mut edges = Vec::new();
        let mut links = vec![0usize; pages.len()];
        for row in rows {
            let (src, target) = row?;
            let Some(&a) = at.get(src.as_str()) else {
                continue;
            };
            let Some(&b) = resolver.resolve(&target).and_then(|t| at.get(t.as_str())) else {
                continue;
            };
            if a == b || !seen.insert((a.min(b), a.max(b))) {
                continue;
            }
            edges.push((a, b));
            links[a] += 1;
            links[b] += 1;
        }
        Ok(WikiGraph {
            nodes: pages
                .into_iter()
                .zip(links)
                .map(|(page, links)| WikiGraphNode { page, links })
                .collect(),
            edges,
        })
    }

    /// Most recently updated pages, optionally in one vault.
    pub fn recent(&self, vault: Option<&str>, limit: usize) -> Result<Vec<PageRef>> {
        let mut stmt = self.conn.prepare(
            "SELECT path, vault, kind, title_en, title_fa, summary, updated FROM pages
             WHERE (?1 IS NULL OR vault = ?1) AND path NOT LIKE '%/index.md'
             ORDER BY updated DESC, mtime DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![vault, limit as i64], row_to_ref)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// One level of the page tree under `dir` (e.g. `vaults/life` or `vaults/life/people`):
    /// sub-folders with their page counts, then pages, both sorted. Loaded lazily per folder.
    pub fn list_dir(&self, dir: &str) -> Result<DirListing> {
        let prefix = format!("{}/", dir.trim_end_matches('/'));
        let mut stmt = self.conn.prepare(
            "SELECT path, vault, kind, title_en, title_fa, summary, updated FROM pages
             WHERE substr(path, 1, length(?1)) = ?1 ORDER BY path",
        )?;
        let all: Vec<PageRef> = stmt
            .query_map([&prefix], row_to_ref)?
            .collect::<Result<_, _>>()?;
        let mut folders: std::collections::BTreeMap<String, usize> = Default::default();
        let mut pages = Vec::new();
        for r in all {
            let rest = &r.path[prefix.len()..];
            match rest.split_once('/') {
                Some((folder, _)) => *folders.entry(format!("{prefix}{folder}")).or_default() += 1,
                None if rest != "index.md" => pages.push(r),
                None => {}
            }
        }
        pages.sort_by_key(|p| p.title_en.to_lowercase());
        Ok(DirListing {
            folders: folders
                .into_iter()
                .map(|(path, pages)| Folder { path, pages })
                .collect(),
            pages,
        })
    }

    pub fn page_ref(&self, path: &str) -> Result<PageRef> {
        Ok(self
            .conn
            .query_row(
                "SELECT path, vault, kind, title_en, title_fa, summary, updated FROM pages WHERE path = ?1",
                [path],
                row_to_ref,
            )
            .optional()?
            .unwrap_or_else(|| PageRef {
                path: path.to_owned(),
                vault: pages::vault_of(path).unwrap_or_default().to_owned(),
                title_en: crate::wiki::slug_of(path).to_owned(),
                title_fa: String::new(),
                kind: String::new(),
                summary: String::new(),
                updated: String::new(),
            }))
    }

    pub fn count(&self) -> Result<usize> {
        Ok(self
            .conn
            .query_row("SELECT count(*) FROM pages", [], |r| r.get::<_, i64>(0))?
            as usize)
    }
}

/// A page as listed in the Wiki tab, backlinks and the graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageRef {
    pub path: String,
    pub vault: String,
    pub kind: String,
    pub title_en: String,
    pub title_fa: String,
    pub summary: String,
    pub updated: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    pub path: String,
    /// Pages anywhere below this folder.
    pub pages: usize,
}

/// One level of the page tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirListing {
    pub folders: Vec<Folder>,
    pub pages: Vec<PageRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub page: PageRef,
    pub depth: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WikiGraphNode {
    pub page: PageRef,
    /// Distinct pages this one links to or is linked from.
    pub links: usize,
}

/// Every page and the links between them. Edges are (source, target) indexes into `nodes`, one
/// per linked pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WikiGraph {
    pub nodes: Vec<WikiGraphNode>,
    pub edges: Vec<(usize, usize)>,
}

fn row_to_ref(r: &rusqlite::Row<'_>) -> rusqlite::Result<PageRef> {
    Ok(PageRef {
        path: r.get(0)?,
        vault: r.get(1)?,
        kind: r.get(2)?,
        title_en: r.get(3)?,
        title_fa: r.get(4)?,
        summary: r.get(5)?,
        updated: r.get(6)?,
    })
}

fn upsert(tx: &rusqlite::Transaction<'_>, p: &Page, mtime: i64, size: i64) -> Result<()> {
    delete(tx, &p.path)?;
    for l in crate::wiki::links(&p.body) {
        if l.target.is_empty() || l.target.starts_with("raw/") {
            continue;
        }
        let slug = l
            .target
            .rsplit('/')
            .next()
            .unwrap_or(&l.target)
            .to_lowercase();
        tx.execute(
            "INSERT INTO links (src, target, slug) VALUES (?1, ?2, ?3)",
            params![p.path, l.target, slug],
        )?;
    }
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
    tx.execute("DELETE FROM links WHERE src = ?1", [path])?;
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
    fn backlinks_graph_recent_and_tree() {
        let (_d, lib) = lib_in();
        write(
            &lib,
            "vaults/life/people/sara.md",
            "Sara",
            "سارا",
            &[],
            "Cousin.",
        );
        write(
            &lib,
            "vaults/life/journal/2026/2026-09-23.md",
            "23 Sep",
            "۱ مهر",
            &[],
            "Called [[sara|Sara]] about [[people/ali]]. ([[raw/2026/09/23/x|voice]])",
        );
        write(
            &lib,
            "vaults/life/people/ali.md",
            "Ali",
            "علی",
            &[],
            "Friend of [[sara]].",
        );
        write(
            &lib,
            "vaults/health/profile.md",
            "Health profile",
            "پروفایل سلامت",
            &[],
            "See [[2026-09-23]].",
        );
        let mut idx = SearchIndex::open(&lib).unwrap();
        idx.refresh(&lib).unwrap();

        let paths = |v: Vec<PageRef>| v.into_iter().map(|r| r.path).collect::<Vec<_>>();
        assert_eq!(
            paths(idx.backlinks("vaults/life/people/sara.md").unwrap()),
            vec![
                "vaults/life/journal/2026/2026-09-23.md",
                "vaults/life/people/ali.md"
            ]
        );
        assert_eq!(
            paths(
                idx.outlinks("vaults/life/journal/2026/2026-09-23.md")
                    .unwrap()
            ),
            vec!["vaults/life/people/ali.md", "vaults/life/people/sara.md"],
            "raw citations are not graph edges"
        );
        let g = idx
            .local_graph("vaults/life/people/sara.md", 2, 50)
            .unwrap();
        assert_eq!(g.nodes.len(), 4);
        assert_eq!(
            g.nodes
                .iter()
                .find(|n| n.page.path == "vaults/health/profile.md")
                .unwrap()
                .depth,
            2
        );
        assert!(g.edges.contains(&(
            "vaults/life/people/ali.md".into(),
            "vaults/life/people/sara.md".into()
        )));
        assert_eq!(
            idx.local_graph("vaults/life/people/sara.md", 2, 2)
                .unwrap()
                .nodes
                .len(),
            2
        );

        write(
            &lib,
            "vaults/life/topics/lonely.md",
            "Lonely",
            "",
            &[],
            "No links.",
        );
        idx.refresh(&lib).unwrap();
        let g = idx.wiki_graph(None).unwrap();
        let name = |i: usize| g.nodes[i].page.path.rsplit('/').next().unwrap().to_owned();
        let mut pairs: Vec<(String, String)> =
            g.edges.iter().map(|&(a, b)| (name(a), name(b))).collect();
        pairs.sort();
        assert_eq!(
            pairs,
            vec![
                ("2026-09-23.md".into(), "ali.md".into()),
                ("2026-09-23.md".into(), "sara.md".into()),
                ("ali.md".into(), "sara.md".into()),
                ("profile.md".into(), "2026-09-23.md".into()),
            ],
            "one edge per linked pair; raw citations left out"
        );
        let links = |p: &str| g.nodes.iter().find(|n| n.page.path == p).unwrap().links;
        assert_eq!(links("vaults/life/journal/2026/2026-09-23.md"), 3);
        assert_eq!(
            links("vaults/life/topics/lonely.md"),
            0,
            "orphans are nodes"
        );
        let life_g = idx.wiki_graph(Some("life")).unwrap();
        assert_eq!(life_g.nodes.len(), 4);
        assert_eq!(life_g.edges.len(), 3, "links out of the vault are dropped");
        std::fs::remove_file(lib.path("vaults/life/topics/lonely.md")).unwrap();
        idx.refresh(&lib).unwrap();

        let life = idx.list_dir("vaults/life").unwrap();
        assert_eq!(
            life.folders
                .iter()
                .map(|f| (f.path.as_str(), f.pages))
                .collect::<Vec<_>>(),
            vec![("vaults/life/journal", 1), ("vaults/life/people", 2)]
        );
        assert!(life.pages.is_empty(), "index.md is not listed");
        let people = idx.list_dir("vaults/life/people").unwrap().pages;
        assert_eq!(
            paths(people),
            vec!["vaults/life/people/ali.md", "vaults/life/people/sara.md"]
        );
        assert_eq!(idx.recent(Some("health"), 5).unwrap().len(), 1);

        // Editing a page replaces its links.
        write(
            &lib,
            "vaults/life/people/ali.md",
            "Ali",
            "علی",
            &[],
            "No links now.",
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
        idx.refresh(&lib).unwrap();
        assert_eq!(
            idx.backlinks("vaults/life/people/sara.md").unwrap().len(),
            1
        );
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
