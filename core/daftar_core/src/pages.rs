//! The wiki as a set of pages on disk: enumeration, link resolution and generated indexes (§4.4).

use std::collections::{BTreeMap, HashMap};
use std::fs;

use crate::fsutil::atomic_write;
use crate::index_md::GENERATED_MARKER;
use crate::library::Library;
use crate::wiki::{self, Page};
use crate::{Result, layout};

/// Repo-relative paths of all wiki pages (generated vault indexes excluded).
pub fn page_paths(lib: &Library) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut stack = vec![layout::VAULTS_DIR.to_owned()];
    while let Some(rel) = stack.pop() {
        let Ok(entries) = fs::read_dir(lib.path(&rel)) else {
            continue;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let child = format!("{rel}/{name}");
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                stack.push(child);
            } else if name.ends_with(".md") && !is_generated_index(&child) {
                out.push(child);
            }
        }
    }
    out.sort();
    Ok(out)
}

pub fn is_generated_index(path: &str) -> bool {
    path.starts_with("vaults/") && path.ends_with("/index.md") && path.matches('/').count() == 2
}

pub fn vault_of(path: &str) -> Option<&str> {
    path.strip_prefix("vaults/")?.split('/').next()
}

pub fn read(lib: &Library, path: &str) -> Result<Page> {
    wiki::parse(path, &fs::read_to_string(lib.path(path))?)
}

pub fn load_all(lib: &Library) -> Result<Vec<Page>> {
    let mut out = Vec::new();
    for p in page_paths(lib)? {
        match read(lib, &p) {
            Ok(page) => out.push(page),
            Err(e) => tracing::warn!("unreadable page {p}: {e}"),
        }
    }
    Ok(out)
}

/// Resolves wikilink targets the way Obsidian does: exact repo path, else unique file name.
#[derive(Debug, Default, Clone)]
pub struct Resolver {
    by_slug: HashMap<String, Vec<String>>,
    paths: std::collections::HashSet<String>,
}

impl Resolver {
    pub fn new<'a>(paths: impl IntoIterator<Item = &'a str>) -> Self {
        let mut r = Self::default();
        for p in paths {
            r.add(p);
        }
        r
    }

    pub fn add(&mut self, path: &str) {
        let no_ext = path.trim_end_matches(".md").to_owned();
        self.paths.insert(no_ext);
        self.by_slug
            .entry(wiki::slug_of(path).to_lowercase())
            .or_default()
            .push(path.to_owned());
    }

    /// Repo path (with `.md`) a link target points to.
    pub fn resolve(&self, target: &str) -> Option<String> {
        let t = target.trim().trim_end_matches(".md");
        if self.paths.contains(t) {
            return Some(format!("{t}.md"));
        }
        let slug = t.rsplit('/').next().unwrap_or(t).to_lowercase();
        let candidates = self.by_slug.get(&slug)?;
        if candidates.len() == 1 {
            return Some(candidates[0].clone());
        }
        // Ambiguous short name: only a partial path (`people/sara`) can disambiguate.
        if !t.contains('/') {
            return None;
        }
        let suffix = format!("/{t}");
        let mut matches = candidates
            .iter()
            .filter(|c| c.trim_end_matches(".md").ends_with(&suffix));
        match (matches.next(), matches.next()) {
            (Some(one), None) => Some(one.clone()),
            _ => None,
        }
    }
}

/// Resolver over all pages plus raw captures (citations link to `raw/...`).
pub fn resolver(lib: &Library) -> Result<Resolver> {
    let mut r = Resolver::new(page_paths(lib)?.iter().map(String::as_str));
    for p in raw_paths(lib)? {
        r.add(&p);
    }
    Ok(r)
}

pub fn raw_paths(lib: &Library) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut stack = vec![layout::RAW_DIR.to_owned()];
    while let Some(rel) = stack.pop() {
        let Ok(entries) = fs::read_dir(lib.path(&rel)) else {
            continue;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let child = format!("{rel}/{name}");
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if name != "assets" {
                    stack.push(child);
                }
            } else if name.ends_with(".md") && !name.starts_with('.') {
                out.push(child);
            }
        }
    }
    Ok(out)
}

/// Deterministic `vaults/<vault>/index.md` from page frontmatter. Grouped by type, sorted by title.
pub fn generate_index(vault_title_en: &str, vault_title_fa: &str, pages: &[&Page]) -> String {
    let mut by_type: BTreeMap<&str, Vec<&Page>> = BTreeMap::new();
    for p in pages {
        let t = if p.meta.kind.is_empty() {
            "page"
        } else {
            p.meta.kind.as_str()
        };
        by_type.entry(t).or_default().push(p);
    }
    let mut out = format!("# {vault_title_en} · {vault_title_fa}\n\n{GENERATED_MARKER}\n");
    if pages.is_empty() {
        out.push_str("\n_No pages yet._\n");
        return out;
    }
    for (t, mut ps) in by_type {
        ps.sort_by_key(|p| p.title("en").to_lowercase());
        out.push_str(&format!("\n## {t}\n\n"));
        for p in ps {
            let path = p.path.trim_end_matches(".md");
            let title = match (p.meta.title.en.as_str(), p.meta.title.fa.as_str()) {
                ("", "") => p.slug().to_owned(),
                (en, "") => en.to_owned(),
                ("", fa) => fa.to_owned(),
                (en, fa) if en == fa => en.to_owned(),
                (en, fa) => format!("{en} · {fa}"),
            };
            let summary = if p.meta.summary.is_empty() {
                String::new()
            } else {
                format!(" — {}", p.meta.summary.trim())
            };
            let n = p.meta.sources.len();
            let updated = if p.meta.updated.is_empty() {
                String::new()
            } else {
                format!(" · {}", p.meta.updated)
            };
            out.push_str(&format!(
                "- [[{path}|{title}]]{summary} _({n} src{updated})_\n"
            ));
        }
    }
    out
}

/// Regenerates the index of each vault in `vaults`; returns the paths that changed.
pub fn regenerate_indexes(lib: &Library, vaults: &[String]) -> Result<Vec<String>> {
    let config = lib.config()?;
    let pages = load_all(lib)?;
    let mut changed = Vec::new();
    for v in vaults {
        let Some(vc) = config.vault(v) else { continue };
        let ps: Vec<&Page> = pages
            .iter()
            .filter(|p| vault_of(&p.path) == Some(v.as_str()))
            .collect();
        let text = generate_index(&vc.title.en, &vc.title.fa, &ps);
        let rel = layout::vault_index(v);
        let current = fs::read_to_string(lib.path(&rel)).unwrap_or_default();
        if current != text {
            atomic_write(&lib.path(&rel), text.as_bytes())?;
            changed.push(rel);
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolver_prefers_exact_paths_and_unique_names() {
        let r = Resolver::new([
            "vaults/life/people/sara.md",
            "vaults/stories/x/characters/sara.md",
            "vaults/health/profile.md",
            "raw/2026/09/23/a.md",
        ]);
        assert_eq!(
            r.resolve("profile").as_deref(),
            Some("vaults/health/profile.md")
        );
        assert_eq!(r.resolve("sara"), None, "ambiguous");
        assert_eq!(
            r.resolve("people/sara").as_deref(),
            Some("vaults/life/people/sara.md")
        );
        assert_eq!(
            r.resolve("raw/2026/09/23/a").as_deref(),
            Some("raw/2026/09/23/a.md")
        );
        assert_eq!(r.resolve("nope"), None);
    }

    #[test]
    fn index_is_deterministic_and_grouped() {
        let a = wiki::parse("vaults/life/people/sara.md", "---\ntype: person\ntitle: { en: \"Sara\", fa: \"سارا\" }\nsummary: \"Cousin.\"\nsources: [a, b]\nupdated: 2026-09-23\n---\nx").unwrap();
        let b = wiki::parse(
            "vaults/life/concerns/career.md",
            "---\ntype: concern\ntitle: { en: \"Career\", fa: \"\" }\n---\nx",
        )
        .unwrap();
        let out = generate_index("Life", "زندگی", &[&a, &b]);
        assert!(out.contains("## concern\n\n- [[vaults/life/concerns/career|Career]] _(0 src)_"));
        assert!(out.contains(
            "- [[vaults/life/people/sara|Sara · سارا]] — Cousin. _(2 src · 2026-09-23)_"
        ));
        assert!(out.find("## concern").unwrap() < out.find("## person").unwrap());
    }
}
