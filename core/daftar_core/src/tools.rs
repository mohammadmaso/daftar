//! Tools exposed to the model (§6.2). Every write goes to the op's `Changeset`; rules that must
//! hold regardless of the model (protected paths, vault isolation, human text, stale reads) are
//! enforced here and answered with an explanatory error so the model can correct itself.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use base64::Engine;
use git2::Repository;
use jiff::Zoned;
use serde_json::{Value, json};

use crate::changeset::Changeset;
use crate::config::Config;
use crate::library::Library;
use crate::providers::ToolSpec;
use crate::raw::{self, RawItem};
use crate::review::{ReviewItem, ReviewKind};
use crate::search::SearchIndex;
use crate::wiki::{self, PageMeta};
use crate::{layout, ledger, pages};

/// Where an op may write (§3.2 isolation, enforced in code).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Ingest of a personal capture: anywhere in vaults except `stories/`.
    Personal,
    /// Fiction: only inside `vaults/stories/<story>/`.
    Story(String),
    /// Read-only (Ask).
    ReadOnly,
}

pub struct ToolOutput {
    pub text: String,
    /// Image to show the model after the tool result (asset_view).
    pub image: Option<(String, String)>,
}

impl ToolOutput {
    fn text(s: impl Into<String>) -> Self {
        Self {
            text: s.into(),
            image: None,
        }
    }
    fn err(s: impl Into<String>) -> Self {
        Self {
            text: format!("ERROR: {}", s.into()),
            image: None,
        }
    }
}

pub struct OpContext<'a> {
    pub lib: &'a Library,
    pub config: Config,
    pub cs: Changeset,
    pub now: Zoned,
    pub op_id: String,
    pub device: String,
    pub scope: Scope,
    /// The capture being ingested, if any.
    pub source: Option<RawItem>,
    /// Raw captures the model may cite (the source plus any it opened).
    pub citable: BTreeMap<String, RawItem>,
    search: Option<SearchIndex>,
    human_lines: HashMap<String, BTreeSet<usize>>,
}

impl<'a> OpContext<'a> {
    pub fn new(
        lib: &'a Library,
        now: Zoned,
        op_id: String,
        device: String,
        scope: Scope,
        source: Option<RawItem>,
    ) -> crate::Result<Self> {
        let config = lib.config()?;
        let mut citable = BTreeMap::new();
        if let Some(s) = &source {
            citable.insert(s.meta.id.clone(), s.clone());
        }
        let search = SearchIndex::open(lib)
            .and_then(|mut s| s.refresh(lib).map(|_| s))
            .ok();
        Ok(Self {
            lib,
            config,
            cs: Changeset::default(),
            now,
            op_id,
            device,
            scope,
            source,
            citable,
            search,
            human_lines: HashMap::new(),
        })
    }

    fn today(&self) -> String {
        self.now.strftime("%Y-%m-%d").to_string()
    }

    /// Normalises a model path: accepts `people/sara`, `life/people/sara.md`, `vaults/life/…`.
    fn norm_path(&self, p: &str) -> String {
        let p = p.trim().trim_start_matches('/').replace('\\', "/");
        let p = if p.ends_with(".md") {
            p
        } else {
            format!("{p}.md")
        };
        if p.starts_with("vaults/")
            || p.starts_with("raw/")
            || p.starts_with(".daftar/")
            || p == layout::SCHEMA_FILE
        {
            p
        } else if self
            .config
            .vault(p.split('/').next().unwrap_or(""))
            .is_some()
        {
            format!("vaults/{p}")
        } else {
            p
        }
    }

    fn check_writable(&self, path: &str) -> Result<(), String> {
        if layout::is_protected_from_ai(path) {
            return Err(format!(
                "{path} is managed by the app and cannot be written."
            ));
        }
        let Some(vault) = pages::vault_of(path) else {
            return Err("Pages live under vaults/<vault>/.".into());
        };
        if self.config.vault(vault).is_none() {
            return Err(format!("There is no vault called '{vault}'."));
        }
        match &self.scope {
            Scope::ReadOnly => Err("This operation cannot change the wiki.".into()),
            Scope::Personal if vault == "stories" => {
                Err("This capture is not fiction; nothing may be written into stories/.".into())
            }
            Scope::Story(story) if !path.starts_with(&format!("vaults/stories/{story}/")) => {
                Err(format!(
                    "This capture is fiction for the story '{story}'; only vaults/stories/{story}/ may be written. Nothing about the user may come from a story."
                ))
            }
            _ => Ok(()),
        }
    }

    /// 1-based line numbers last written by a human (per git blame), for human-text protection.
    fn human_lines(&mut self, path: &str) -> &BTreeSet<usize> {
        if !self.human_lines.contains_key(path) {
            let set = human_lines_at_head(self.lib, path);
            self.human_lines.insert(path.to_owned(), set);
        }
        &self.human_lines[path]
    }

    // ─────────────────────────── dispatch ───────────────────────────

    pub fn call(&mut self, name: &str, args: &Value) -> ToolOutput {
        let r = match name {
            "index_read" => self.index_read(args),
            "search" => self.search(args),
            "page_read" => self.page_read(args),
            "page_create" => self.page_create(args),
            "page_edit" => self.page_edit(args),
            "claim_propose" => self.claim_propose(args),
            "claim_supersede" => self.claim_supersede(args),
            "raw_read" => self.raw_read(args),
            "asset_view" => return self.asset_view(args),
            "link_suggest" => self.link_suggest(args),
            "review_add" => self.review_add(args),
            other => Err(format!("Unknown tool '{other}'.")),
        };
        match r {
            Ok(t) => ToolOutput::text(t),
            Err(e) => ToolOutput::err(e),
        }
    }

    fn index_read(&self, a: &Value) -> Result<String, String> {
        let vaults: Vec<String> = match a["vault"].as_str() {
            Some(v) => vec![v.to_owned()],
            None => self.config.active_vaults().map(|v| v.id.clone()).collect(),
        };
        let mut out = String::new();
        for v in vaults {
            let text = std::fs::read_to_string(self.lib.path(&layout::vault_index(&v)))
                .map_err(|_| format!("No index for vault '{v}'."))?;
            out.push_str(&text);
            out.push('\n');
        }
        Ok(out)
    }

    fn search(&self, a: &Value) -> Result<String, String> {
        let q = a["query"].as_str().ok_or("query is required")?;
        let vaults: Vec<String> = a["vault"]
            .as_str()
            .map(|v| vec![v.to_owned()])
            .unwrap_or_default();
        let kinds: Vec<String> = a["types"]
            .as_array()
            .map(|t| {
                t.iter()
                    .filter_map(|x| x.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        let limit = a["limit"].as_u64().unwrap_or(8).min(25) as usize;
        let Some(idx) = &self.search else {
            return Ok("Search is unavailable; use index_read.".into());
        };
        let mut hits = idx
            .search(q, &vaults, &kinds, limit)
            .map_err(|e| e.to_string())?;
        // Pages created in this op are not in the index yet.
        for (p, c) in &self.cs.files {
            if let (Some(c), true) = (c, self.cs.created.contains(p))
                && crate::normalize::normalize(c).contains(&crate::normalize::normalize(q))
            {
                hits.insert(
                    0,
                    crate::search::Hit {
                        path: p.clone(),
                        vault: pages::vault_of(p).unwrap_or_default().into(),
                        kind: String::new(),
                        title_en: String::new(),
                        title_fa: String::new(),
                        summary: "(created in this operation)".into(),
                        snippet: String::new(),
                        updated: String::new(),
                        score: 0.0,
                    },
                );
            }
        }
        if hits.is_empty() {
            return Ok(format!("No pages match '{q}'."));
        }
        Ok(hits
            .iter()
            .map(|h| {
                format!(
                    "- {} | {} · {} | {} | {}\n  {}",
                    h.path, h.title_en, h.title_fa, h.kind, h.summary, h.snippet
                )
            })
            .collect::<Vec<_>>()
            .join("\n"))
    }

    fn page_read(&mut self, a: &Value) -> Result<String, String> {
        let path = self.norm_path(a["path"].as_str().ok_or("path is required")?);
        let text = self
            .cs
            .read(self.lib, &path)
            .ok_or_else(|| format!("{path} does not exist. Use search or page_create."))?;
        let n = text.lines().count();
        let from = a["from_line"].as_u64().unwrap_or(1) as usize;
        let to = a["to_line"].as_u64().map(|x| x as usize).unwrap_or(n);
        let human = self.human_lines(&path).clone();
        let note = if human.is_empty() {
            String::new()
        } else {
            format!(
                "human-written lines (append/link only, never rewrite): {}\n",
                compress_ranges(&human)
            )
        };
        Ok(format!(
            "path: {path}\nhash: {}\nlines: {n}\n{note}\n{}",
            wiki::content_hash(&text),
            wiki::line_numbered(&text, from, to)
        ))
    }

    fn page_create(&mut self, a: &Value) -> Result<String, String> {
        let raw = a["path"]
            .as_str()
            .ok_or("path is required, e.g. vaults/life/people/sara")?;
        let proposed = self.norm_path(raw);
        let (dir, file) = proposed.rsplit_once('/').ok_or("path needs a folder")?;
        let dir: String = dir
            .split('/')
            .map(|s| {
                if s == "vaults" {
                    s.to_owned()
                } else {
                    wiki::sanitize_slug(s)
                }
            })
            .collect::<Vec<_>>()
            .join("/");
        let slug = wiki::sanitize_slug(file.trim_end_matches(".md"));
        let path = format!("{dir}/{slug}.md");
        self.check_writable(&path)?;
        if self.cs.exists(self.lib, &path) {
            return Err(format!("{path} already exists; read it and use page_edit."));
        }
        let fm = &a["frontmatter"];
        let kind = fm["type"].as_str().unwrap_or_default().to_owned();
        if !wiki::PAGE_TYPES.contains(&kind.as_str()) {
            return Err(format!(
                "type must be one of: {}.",
                wiki::PAGE_TYPES.join(", ")
            ));
        }
        let title = crate::config::Bilingual {
            en: fm
                .pointer("/title/en")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_owned(),
            fa: fm
                .pointer("/title/fa")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_owned(),
        };
        if title.en.is_empty() || title.fa.is_empty() {
            return Err("title needs both en and fa.".into());
        }
        let aliases: Vec<String> = fm["aliases"]
            .as_array()
            .map(|x| {
                x.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        // Duplicate guard (§15): the same slug or alias elsewhere means the page already exists.
        if let Some(dupe) = self.duplicate_of(&slug, &title, &aliases) {
            return Err(format!(
                "A page for this already exists: {dupe}. Read and update it instead of creating a near-duplicate."
            ));
        }
        let meta = PageMeta {
            id: crate::ids::new_id().to_string(),
            kind,
            vault: pages::vault_of(&path).unwrap_or_default().to_owned(),
            title,
            aliases,
            summary: fm["summary"].as_str().unwrap_or_default().trim().to_owned(),
            sources: self.source_ids(),
            created: self.today(),
            updated: self.today(),
            status: if fm["status"].as_str() == Some("stub") {
                "stub".into()
            } else {
                "active".into()
            },
            extra: Default::default(),
        };
        let body = a["body"].as_str().unwrap_or_default();
        let doc = wiki::render(&meta, body);
        self.cs.write(self.lib, &path, doc.clone());
        Ok(format!(
            "created\npath: {path}\nhash: {}\n",
            wiki::content_hash(&doc)
        ))
    }

    fn duplicate_of(
        &self,
        slug: &str,
        title: &crate::config::Bilingual,
        aliases: &[String],
    ) -> Option<String> {
        let norm = |s: &str| crate::normalize::normalize(s).trim().to_owned();
        let mut names: BTreeSet<String> = aliases.iter().map(|a| norm(a)).collect();
        names.insert(norm(&title.en));
        names.insert(norm(&title.fa));
        names.remove("");
        let scope_prefix = match &self.scope {
            Scope::Story(s) => format!("vaults/stories/{s}/"),
            _ => "vaults/".into(),
        };
        let in_scope = |p: &str| {
            p.starts_with(&scope_prefix)
                && (matches!(self.scope, Scope::Story(_)) || !p.starts_with("vaults/stories/"))
        };
        for p in pages::load_all(self.lib).ok()? {
            if !in_scope(&p.path) {
                continue;
            }
            if p.slug() == slug {
                return Some(p.path);
            }
            let mut theirs: BTreeSet<String> = p.meta.aliases.iter().map(|a| norm(a)).collect();
            theirs.insert(norm(&p.meta.title.en));
            theirs.insert(norm(&p.meta.title.fa));
            if theirs.intersection(&names).next().is_some() {
                return Some(p.path);
            }
        }
        None
    }

    fn source_ids(&self) -> Vec<String> {
        self.source.iter().map(|s| s.meta.id.clone()).collect()
    }

    fn page_edit(&mut self, a: &Value) -> Result<String, String> {
        let path = self.norm_path(a["path"].as_str().ok_or("path is required")?);
        self.check_writable(&path)?;
        let current = self
            .cs
            .read(self.lib, &path)
            .ok_or_else(|| format!("{path} does not exist; use page_create."))?;
        let base = a["base_hash"].as_str().unwrap_or_default();
        if base != wiki::content_hash(&current) {
            return Err(format!(
                "{path} changed since you read it (stale base_hash). Call page_read again and redo the edit."
            ));
        }
        let edits = a["edits"].as_array().ok_or("edits must be a list")?;
        let mut lines: Vec<String> = current.lines().map(str::to_owned).collect();
        let human = self.human_lines(&path).clone();

        // Line edits refer to the base version: apply bottom-up so earlier numbers stay valid.
        let mut line_edits: Vec<(usize, &Value)> = Vec::new();
        let mut other: Vec<&Value> = Vec::new();
        for e in edits {
            match e["op"].as_str() {
                Some("replace_lines") => {
                    line_edits.push((e["from"].as_u64().unwrap_or(0) as usize, e))
                }
                Some("insert_after_line") => {
                    line_edits.push((e["line"].as_u64().unwrap_or(0) as usize, e))
                }
                Some("append_to_section") | Some("add_frontmatter_values") => other.push(e),
                other => {
                    return Err(format!(
                        "Unknown edit op {other:?}. Use replace_lines, insert_after_line, append_to_section, add_frontmatter_values."
                    ));
                }
            }
        }
        line_edits.sort_by_key(|(l, _)| std::cmp::Reverse(*l));
        let fm_end = frontmatter_end(&lines);
        for (_, e) in line_edits {
            let text: Vec<String> = e["text"]
                .as_str()
                .unwrap_or_default()
                .lines()
                .map(str::to_owned)
                .collect();
            match e["op"].as_str() {
                Some("replace_lines") => {
                    let from = e["from"].as_u64().unwrap_or(0) as usize;
                    let to = e["to"].as_u64().unwrap_or(from as u64) as usize;
                    if from == 0 || to < from || to > lines.len() {
                        return Err(format!(
                            "replace_lines {from}-{to} is outside 1-{}.",
                            lines.len()
                        ));
                    }
                    if from <= fm_end {
                        return Err(
                            "Frontmatter lines cannot be replaced; use add_frontmatter_values."
                                .into(),
                        );
                    }
                    if let Some(h) =
                        (from..=to).find(|l| human.contains(l) && !lines[l - 1].trim().is_empty())
                    {
                        return Err(format!(
                            "Line {h} was written by the user. You may append or add links, but not rewrite their text. If it is wrong, call review_add with kind 'question'."
                        ));
                    }
                    lines.splice(from - 1..to, text);
                }
                _ => {
                    let at = e["line"].as_u64().unwrap_or(0) as usize;
                    if at > lines.len() {
                        return Err(format!(
                            "insert_after_line {at} is beyond the last line {}.",
                            lines.len()
                        ));
                    }
                    let at = at.max(fm_end);
                    lines.splice(at..at, text);
                }
            }
        }
        let mut body_text = lines.join("\n");
        for e in other {
            match e["op"].as_str() {
                Some("append_to_section") => {
                    let heading = e["heading"]
                        .as_str()
                        .ok_or("append_to_section needs heading")?;
                    let text = e["text"].as_str().unwrap_or_default();
                    body_text = append_to_section(&body_text, heading, text);
                }
                _ => {
                    // Re-parse after line edits, then merge values.
                    let mut page = wiki::parse(&path, &body_text).map_err(|e| e.to_string())?;
                    if let Some(al) = e["aliases"].as_array() {
                        for x in al.iter().filter_map(Value::as_str) {
                            if !page.meta.aliases.iter().any(|a| a == x) {
                                page.meta.aliases.push(x.to_owned());
                            }
                        }
                    }
                    if let Some(s) = e["summary"].as_str() {
                        page.meta.summary = s.trim().to_owned();
                    }
                    if let Some(t) = e.pointer("/title/en").and_then(Value::as_str)
                        && page.meta.title.en.is_empty()
                    {
                        page.meta.title.en = t.to_owned();
                    }
                    if let Some(t) = e.pointer("/title/fa").and_then(Value::as_str)
                        && page.meta.title.fa.is_empty()
                    {
                        page.meta.title.fa = t.to_owned();
                    }
                    body_text = page.render();
                }
            }
        }
        let mut page = wiki::parse(&path, &body_text)
            .map_err(|e| format!("The edit broke the frontmatter: {e}"))?;
        page.meta.updated = self.today();
        for s in self.source_ids() {
            if !page.meta.sources.contains(&s) {
                page.meta.sources.push(s);
            }
        }
        let doc = page.render();
        self.cs.write(self.lib, &path, doc.clone());
        Ok(format!(
            "edited\npath: {path}\nhash: {}\nlines: {}\n",
            wiki::content_hash(&doc),
            doc.lines().count()
        ))
    }

    fn cite(&self, ids: &[String]) -> Result<Vec<(String, String)>, String> {
        let ids: Vec<String> = if ids.is_empty() {
            self.source_ids()
        } else {
            ids.to_vec()
        };
        ids.iter()
            .map(|id| {
                let item = self.citable.get(id).ok_or_else(|| format!("Unknown source {id}; cite the capture being filed or one you opened with raw_read."))?;
                Ok((item.path.clone(), source_label(item)))
            })
            .collect()
    }

    fn claim_propose(&mut self, a: &Value) -> Result<String, String> {
        let path = self.norm_path(a["page"].as_str().ok_or("page is required")?);
        self.check_writable(&path)?;
        let text = a["text"].as_str().ok_or("text is required")?.trim();
        let kind = a["kind"].as_str().unwrap_or("inferred");
        let ids: Vec<String> = a["sources"]
            .as_array()
            .map(|x| {
                x.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        let sources = self.cite(&ids)?;
        // §3.4: only literally stated facts may be confirmed; everything inferred waits for review.
        let status = if kind == "stated" {
            "confirmed"
        } else {
            "proposed"
        };
        let confidence = if status == "proposed" {
            Some(a["confidence"].as_str().unwrap_or("medium"))
        } else {
            None
        };
        let id = format!("c-{}", crate::ids::new_id());
        let line = wiki::claim_line(text, status, confidence, &sources, &id);
        let current = self
            .cs
            .read(self.lib, &path)
            .ok_or_else(|| format!("{path} does not exist; create it first."))?;
        let section = a["section"].as_str().unwrap_or("Claims");
        let updated = append_to_section(&current, section, &line);
        let mut page = wiki::parse(&path, &updated).map_err(|e| e.to_string())?;
        page.meta.updated = self.today();
        for s in self.source_ids() {
            if !page.meta.sources.contains(&s) {
                page.meta.sources.push(s);
            }
        }
        let doc = page.render();
        self.cs.write(self.lib, &path, doc.clone());
        self.cs.claims_added.push(id.clone());
        self.cs.claim_ids_from_tools.insert(id.clone());
        if status == "proposed" {
            self.cs.review_items.push(ReviewItem::new(
                ReviewKind::Claim,
                &self.device,
                &self.now,
                Some(self.op_id.clone()),
                json!({"page": path, "claim_id": id, "text": text, "confidence": confidence}),
            ));
        }
        Ok(format!(
            "claim {id} added as {status} to {path}\npath: {path}\nhash: {}\n",
            wiki::content_hash(&doc)
        ))
    }

    fn claim_supersede(&mut self, a: &Value) -> Result<String, String> {
        let old = a["claim_id"]
            .as_str()
            .ok_or("claim_id is required")?
            .trim_start_matches('^');
        let new_text = a["new_text"].as_str().ok_or("new_text is required")?;
        let ids: Vec<String> = a["sources"]
            .as_array()
            .map(|x| {
                x.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        let sources = self.cite(&ids)?;
        let marker = format!("^{old}");
        let page_path = pages::page_paths(self.lib)
            .map_err(|e| e.to_string())?
            .into_iter()
            .chain(self.cs.created.iter().cloned())
            .find(|p| {
                self.cs
                    .read(self.lib, p)
                    .is_some_and(|t| t.lines().any(|l| l.trim_end().ends_with(&marker)))
            })
            .ok_or_else(|| format!("No claim {old} found."))?;
        self.check_writable(&page_path)?;
        let text = self.cs.read(self.lib, &page_path).expect("found above");
        let new_id = format!("c-{}", crate::ids::new_id());
        let mut out = Vec::new();
        for l in text.lines() {
            if l.trim_end().ends_with(&marker) {
                let replaced = l
                    .replace("(status:: confirmed)", "(status:: superseded)")
                    .replace("(status:: proposed)", "(status:: superseded)");
                let with_link = replaced.replacen(
                    &format!(" {marker}"),
                    &format!(" (superseded_by:: [[#^{new_id}]]) {marker}"),
                    1,
                );
                out.push(with_link);
                // Contradictions are never resolved silently: the replacement waits for review.
                out.push(wiki::claim_line(
                    new_text,
                    "proposed",
                    Some("high"),
                    &sources,
                    &new_id,
                ));
            } else {
                out.push(l.to_owned());
            }
        }
        let mut page = wiki::parse(&page_path, &out.join("\n")).map_err(|e| e.to_string())?;
        page.meta.updated = self.today();
        let doc = page.render();
        self.cs.write(self.lib, &page_path, doc.clone());
        self.cs.claims_added.push(new_id.clone());
        self.cs.claim_ids_from_tools.insert(new_id.clone());
        self.cs.review_items.push(ReviewItem::new(
            ReviewKind::Claim,
            &self.device,
            &self.now,
            Some(self.op_id.clone()),
            json!({"page": page_path, "claim_id": new_id, "supersedes": old, "text": new_text}),
        ));
        Ok(format!(
            "{old} superseded by {new_id} (proposed, in Review)\npath: {page_path}\nhash: {}\n",
            wiki::content_hash(&doc)
        ))
    }

    fn raw_read(&mut self, a: &Value) -> Result<String, String> {
        let id = a["id"].as_str().ok_or("id is required")?;
        let ulid: ulid::Ulid = id
            .parse()
            .map_err(|_| "id must be a capture id".to_owned())?;
        let item = raw::find(self.lib, ulid)
            .map_err(|e| e.to_string())?
            .ok_or("No such capture.")?;
        let out = format!(
            "id: {}\npath: {}\nkind: {}\ncaptured_at: {}\n\n{}",
            item.meta.id,
            item.path,
            item.meta.kind.as_str(),
            item.meta.captured_at,
            item.body
        );
        self.citable.insert(item.meta.id.clone(), item);
        Ok(out)
    }

    fn asset_view(&self, a: &Value) -> ToolOutput {
        let Some(p) = a["path"].as_str() else {
            return ToolOutput::err("path is required");
        };
        if !p.starts_with("raw/assets/") {
            return ToolOutput::err("Only capture assets under raw/assets/ can be viewed.");
        }
        match std::fs::read(self.lib.path(p)) {
            Ok(bytes) => ToolOutput {
                text: format!("Image {p} attached below."),
                image: Some((
                    "image/jpeg".into(),
                    base64::engine::general_purpose::STANDARD.encode(bytes),
                )),
            },
            Err(_) => ToolOutput::err(format!("{p} not found.")),
        }
    }

    fn link_suggest(&self, a: &Value) -> Result<String, String> {
        let text = a["text"].as_str().ok_or("text is required")?;
        let norm = crate::normalize::normalize(text);
        let mut found = Vec::new();
        for p in pages::load_all(self.lib).map_err(|e| e.to_string())? {
            let names = [p.meta.title.en.as_str(), p.meta.title.fa.as_str()]
                .into_iter()
                .chain(p.meta.aliases.iter().map(String::as_str));
            for n in names {
                let nn = crate::normalize::normalize(n);
                if nn.chars().count() >= 3 && contains_word(&norm, &nn) {
                    found.push(format!(
                        "\"{n}\" → [[{}|{n}]]",
                        p.path.trim_end_matches(".md")
                    ));
                    break;
                }
            }
        }
        Ok(if found.is_empty() {
            "No known pages mentioned.".into()
        } else {
            found.join("\n")
        })
    }

    fn review_add(&mut self, a: &Value) -> Result<String, String> {
        let kind = match a["kind"].as_str().unwrap_or("question") {
            "question" => ReviewKind::Question,
            "schema" => ReviewKind::Schema,
            "routing" => ReviewKind::Routing,
            "lint" => ReviewKind::Lint,
            k => return Err(format!("Unsupported review kind '{k}'.")),
        };
        let item = ReviewItem::new(
            kind,
            &self.device,
            &self.now,
            Some(self.op_id.clone()),
            a["payload"].clone(),
        );
        let id = item.id.clone();
        self.cs.review_items.push(item);
        Ok(format!("Added to the user's Review queue ({id})."))
    }
}

fn contains_word(hay: &str, needle: &str) -> bool {
    hay.match_indices(needle).any(|(i, _)| {
        let before = hay[..i].chars().next_back();
        let after = hay[i + needle.len()..].chars().next();
        before.is_none_or(|c| !c.is_alphanumeric()) && after.is_none_or(|c| !c.is_alphanumeric())
    })
}

fn frontmatter_end(lines: &[String]) -> usize {
    if lines.first().map(|l| l.trim()) != Some("---") {
        return 0;
    }
    lines
        .iter()
        .skip(1)
        .position(|l| l.trim() == "---")
        .map(|p| p + 2)
        .unwrap_or(0)
}

/// Appends `text` at the end of the section titled `heading` (any level); creates `## heading` at
/// the end of the page if it does not exist.
pub fn append_to_section(doc: &str, heading: &str, text: &str) -> String {
    let lines: Vec<&str> = doc.lines().collect();
    let secs = wiki::sections(doc);
    let target = crate::normalize::normalize(heading.trim_start_matches('#').trim());
    let found = secs
        .iter()
        .filter(|s| crate::normalize::normalize(&s.heading) == target)
        .min_by_key(|s| s.level);
    let mut out: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    match found {
        Some(s) => {
            // Insert before trailing blank lines of the section, but after its content.
            let mut at = s.end.min(out.len());
            // Sub-sections belong to the section: append before the next same-or-higher heading.
            while at > s.start && out[at - 1].trim().is_empty() {
                at -= 1;
            }
            let block: Vec<String> = text.lines().map(str::to_owned).collect();
            out.splice(at..at, block);
        }
        None => {
            while out.last().is_some_and(|l| l.trim().is_empty()) {
                out.pop();
            }
            out.push(String::new());
            out.push(format!("## {}", heading.trim_start_matches('#').trim()));
            out.extend(text.lines().map(str::to_owned));
        }
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}

pub fn source_label(item: &RawItem) -> String {
    let kind = item.meta.kind.as_str().replace('-', " ");
    let date = crate::time::parse_rfc3339(&item.meta.captured_at)
        .map(|t| {
            t.to_zoned(jiff::tz::TimeZone::UTC)
                .strftime("%-d %b")
                .to_string()
        })
        .unwrap_or_default();
    if date.is_empty() {
        kind
    } else {
        format!("{kind} · {date}")
    }
}

fn compress_ranges(set: &BTreeSet<usize>) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut it = set.iter().peekable();
    while let Some(&s) = it.next() {
        let mut e = s;
        while it.peek() == Some(&&(e + 1)) {
            e = *it.next().expect("peeked");
        }
        out.push(if s == e {
            s.to_string()
        } else {
            format!("{s}-{e}")
        });
    }
    out.join(", ")
}

/// 1-based lines of `path` at HEAD last changed by a human commit (no `Op-Id` trailer, not the
/// initial scaffold). Used to protect the user's own text (§5.5).
pub fn human_lines_at_head(lib: &Library, path: &str) -> BTreeSet<usize> {
    let Ok(repo) = Repository::open(lib.root()) else {
        return BTreeSet::new();
    };
    let Ok(blame) = repo.blame_file(std::path::Path::new(path), None) else {
        return BTreeSet::new();
    };
    let mut human_commits: HashMap<git2::Oid, bool> = HashMap::new();
    let mut out = BTreeSet::new();
    for hunk in blame.iter() {
        let oid = hunk.final_commit_id();
        let human = *human_commits.entry(oid).or_insert_with(|| {
            repo.find_commit(oid).is_ok_and(|c| {
                let m = c.message().unwrap_or_default();
                ledger::trailer(m, "Op-Id").is_none() && !m.starts_with("init:")
            })
        });
        if human {
            let start = hunk.final_start_line();
            out.extend(start..start + hunk.lines_in_hunk());
        }
    }
    out
}

/// JSON schemas for the tools (§6.2). `write` = include mutating tools.
pub fn specs(write: bool) -> Vec<ToolSpec> {
    let s = |name: &str, description: &str, parameters: Value| ToolSpec {
        name: name.into(),
        description: description.into(),
        parameters,
    };
    let mut v = vec![
        s(
            "index_read",
            "Read the generated index of one vault (or all). Lists every page with title, summary and source count.",
            json!({"type": "object", "properties": {"vault": {"type": "string"}}}),
        ),
        s(
            "search",
            "Full-text search over the wiki in Persian and English. Returns paths, titles, summaries and a snippet.",
            json!({"type": "object", "properties": {"query": {"type": "string"}, "vault": {"type": "string"}, "types": {"type": "array", "items": {"type": "string"}}, "limit": {"type": "integer"}}, "required": ["query"]}),
        ),
        s(
            "page_read",
            "Read a page with line numbers and its content hash (needed for page_edit).",
            json!({"type": "object", "properties": {"path": {"type": "string"}, "from_line": {"type": "integer"}, "to_line": {"type": "integer"}}, "required": ["path"]}),
        ),
        s(
            "raw_read",
            "Read a raw capture by id (to cite it or check what the user actually said).",
            json!({"type": "object", "properties": {"id": {"type": "string"}}, "required": ["id"]}),
        ),
        s(
            "asset_view",
            "Look at an image attached to a capture (path under raw/assets/).",
            json!({"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}),
        ),
        s(
            "link_suggest",
            "Find known pages (by title or alias) mentioned in a text, to link them.",
            json!({"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]}),
        ),
    ];
    if write {
        v.extend([
            s("page_create", "Create a page. The code fills id, dates, vault and sources. path like vaults/life/people/sara (ASCII kebab-case slug).", json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "frontmatter": {"type": "object", "properties": {
                        "type": {"type": "string", "enum": wiki::PAGE_TYPES},
                        "title": {"type": "object", "properties": {"en": {"type": "string"}, "fa": {"type": "string"}}, "required": ["en", "fa"]},
                        "aliases": {"type": "array", "items": {"type": "string"}},
                        "summary": {"type": "string"},
                        "status": {"type": "string", "enum": ["active", "stub"]}
                    }, "required": ["type", "title", "summary"]},
                    "body": {"type": "string"}
                },
                "required": ["path", "frontmatter", "body"]
            })),
            s("page_edit", "Edit a page by line numbers from your latest page_read. Rejected if base_hash is stale. Ops: replace_lines {from,to,text}, insert_after_line {line,text}, append_to_section {heading,text}, add_frontmatter_values {aliases?, summary?}.", json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "base_hash": {"type": "string"},
                    "edits": {"type": "array", "items": {"type": "object", "properties": {
                        "op": {"type": "string", "enum": ["replace_lines", "insert_after_line", "append_to_section", "add_frontmatter_values"]},
                        "from": {"type": "integer"}, "to": {"type": "integer"}, "line": {"type": "integer"},
                        "heading": {"type": "string"}, "text": {"type": "string"},
                        "aliases": {"type": "array", "items": {"type": "string"}}, "summary": {"type": "string"}
                    }, "required": ["op"]}}
                },
                "required": ["path", "base_hash", "edits"]
            })),
            s("claim_propose", "Add a claim about the user to a page (health, mind, life profile/concerns). kind 'stated' = the user literally said it (confirmed); 'inferred' = anything else (proposed, goes to Review).", json!({
                "type": "object",
                "properties": {"page": {"type": "string"}, "text": {"type": "string"}, "sources": {"type": "array", "items": {"type": "string"}}, "kind": {"type": "string", "enum": ["stated", "inferred"]}, "confidence": {"type": "string", "enum": ["low", "medium", "high"]}, "section": {"type": "string"}},
                "required": ["page", "text", "kind"]
            })),
            s("claim_supersede", "Mark an existing claim superseded by new evidence; the replacement goes to Review.", json!({
                "type": "object",
                "properties": {"claim_id": {"type": "string"}, "new_text": {"type": "string"}, "sources": {"type": "array", "items": {"type": "string"}}},
                "required": ["claim_id", "new_text"]
            })),
        ]);
    }
    v.push(s("review_add", "Ask the user something asynchronously via the Review queue.", json!({"type": "object", "properties": {"kind": {"type": "string", "enum": ["question", "schema"]}, "payload": {"type": "object"}}, "required": ["kind", "payload"]})));
    v
}
