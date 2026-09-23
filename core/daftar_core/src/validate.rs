//! Changeset validator (§6.4). Runs in code after the model finishes; errors are fed back for up to
//! two repair rounds, then the op is aborted.

use std::collections::{BTreeSet, HashMap};

use crate::changeset::Changeset;
use crate::library::Library;
use crate::normalize::normalize;
use crate::pages;
use crate::tools::Scope;
use crate::wiki;
use crate::layout;

pub const MAX_PAGE_LINES: usize = 400;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Sections whose list items are not claims even in claim zones.
const NON_CLAIM_SECTIONS: &[&str] = &[
    "timeline", "related", "see also", "open questions", "questions", "sources", "visits", "results", "log", "notes",
    "خط زمانی", "مرتبط", "پرسش‌ها", "پرسش ها", "منابع", "یادداشت‌ها",
];

pub fn is_claim_zone(path: &str) -> bool {
    path.starts_with("vaults/health/") || path.starts_with("vaults/mind/") || path == "vaults/life/profile.md" || path.starts_with("vaults/life/concerns/")
}

fn added_lines<'a>(old: &str, new: &'a str) -> Vec<(usize, &'a str)> {
    let mut pool: HashMap<&str, usize> = HashMap::new();
    for l in old.lines() {
        *pool.entry(l).or_default() += 1;
    }
    let mut out = Vec::new();
    for (i, l) in new.lines().enumerate() {
        match pool.get_mut(l) {
            Some(n) if *n > 0 => *n -= 1,
            _ => out.push((i + 1, l)),
        }
    }
    out
}

fn removed_lines(old: &str, new: &str) -> Vec<usize> {
    let mut pool: HashMap<&str, usize> = HashMap::new();
    for l in new.lines() {
        *pool.entry(l).or_default() += 1;
    }
    let mut out = Vec::new();
    for (i, l) in old.lines().enumerate() {
        match pool.get_mut(l) {
            Some(n) if *n > 0 => *n -= 1,
            _ => out.push(i + 1),
        }
    }
    out
}

fn is_link_only(line: &str) -> bool {
    let t = line.trim().trim_start_matches(['-', '*']).trim();
    let ls = wiki::links(t);
    if ls.is_empty() {
        return false;
    }
    let mut rest = t.to_owned();
    for l in ls.iter().rev() {
        rest.replace_range(l.start..l.end, "");
    }
    rest.chars().all(|c| !c.is_alphanumeric())
}

pub fn validate(lib: &Library, cs: &Changeset, scope: &Scope, human_lines: &dyn Fn(&str) -> BTreeSet<usize>) -> Report {
    let mut r = Report::default();
    let config = match lib.config() {
        Ok(c) => c,
        Err(e) => {
            r.errors.push(format!("config unreadable: {e}"));
            return r;
        }
    };

    // Resolver over the post-op state.
    let mut resolver = pages::resolver(lib).unwrap_or_default();
    for p in cs.files.keys() {
        resolver.add(p);
    }

    for (path, content) in &cs.files {
        if layout::is_protected_from_ai(path) {
            r.errors.push(format!("{path}: this path may not be written by the assistant."));
            continue;
        }
        let vault = pages::vault_of(path).unwrap_or_default();
        match scope {
            Scope::Personal if vault == "stories" => r.errors.push(format!("{path}: non-fiction content may not be written into stories/.")),
            Scope::Story(s) if !path.starts_with(&format!("vaults/stories/{s}/")) => r.errors.push(format!("{path}: fiction may only be written inside vaults/stories/{s}/.")),
            Scope::ReadOnly => r.errors.push(format!("{path}: read-only operation.")),
            _ => {}
        }
        let Some(new) = content else { continue };
        if !path.starts_with("vaults/") || !path.ends_with(".md") {
            r.errors.push(format!("{path}: only Markdown pages under vaults/ may be written."));
            continue;
        }
        let page = match wiki::parse(path, new) {
            Ok(p) => p,
            Err(e) => {
                r.errors.push(format!("{path}: {e}"));
                continue;
            }
        };
        let stub = page.meta.status == "stub";
        if !wiki::PAGE_TYPES.contains(&page.meta.kind.as_str()) {
            r.errors.push(format!("{path}: frontmatter type '{}' is not a known page type.", page.meta.kind));
        }
        if page.meta.title.en.trim().is_empty() || page.meta.title.fa.trim().is_empty() {
            r.errors.push(format!("{path}: title needs both en and fa."));
        }
        if page.meta.summary.trim().is_empty() && !stub {
            r.errors.push(format!("{path}: summary is empty; it feeds the vault index."));
        }
        if page.meta.vault != vault {
            r.errors.push(format!("{path}: frontmatter vault '{}' does not match the folder.", page.meta.vault));
        }
        if config.vault(vault).is_none() {
            r.errors.push(format!("{path}: unknown vault '{vault}'."));
        }
        let n = new.lines().count();
        if n > MAX_PAGE_LINES {
            r.warnings.push(format!("{path} has {n} lines; consider splitting it."));
        }

        let old = cs.original(lib, path).unwrap_or_default();
        let old_body = wiki::parse(path, &old).map(|p| p.body).unwrap_or_default();
        let added = added_lines(&old_body, &page.body);

        // New links must resolve (or point to stubs created in this op).
        let old_targets: BTreeSet<String> = wiki::links(&old_body).into_iter().map(|l| l.target).collect();
        for l in wiki::links(&page.body) {
            if l.target.is_empty() || old_targets.contains(&l.target) {
                continue;
            }
            if resolver.resolve(&l.target).is_none() {
                r.errors.push(format!("{path}: link [[{}]] does not resolve. Link an existing page or create a stub page.", l.target));
            }
        }

        // Every section with new factual text must cite a raw source.
        if !stub {
            let secs = wiki::sections(&page.body);
            let mut flagged = BTreeSet::new();
            for (ln, line) in &added {
                let t = line.trim();
                if t.is_empty() || t.starts_with('#') || t == "---" || is_link_only(t) {
                    continue;
                }
                let sec = secs.iter().filter(|s| s.start <= *ln && *ln <= s.end).max_by_key(|s| s.level);
                let (start, end, name) = match sec {
                    Some(s) => (s.start, s.end, s.heading.clone()),
                    None => (1, page.body.lines().count(), "(top)".into()),
                };
                let text: Vec<&str> = page.body.lines().skip(start - 1).take(end + 1 - start).collect();
                let cites = text.iter().any(|l| wiki::links(l).iter().any(|k| k.target.starts_with("raw/")));
                if !cites && flagged.insert(name.clone()) {
                    r.errors.push(format!("{path}: section '{name}' gained text without a source citation ([[raw/…|…]])."));
                }
            }
        }

        // Claims only via claim tools; claim syntax in claim zones.
        for (ln, line) in &added {
            if let Some(i) = line.rfind(" ^c-") {
                let id = line[i + 2..].trim();
                if !cs.claim_ids_from_tools.contains(id) && !old_body.contains(&format!("^{id}")) {
                    r.errors.push(format!("{path}:{ln}: claims must be added with claim_propose / claim_supersede, not written by hand."));
                }
            }
        }
        if is_claim_zone(path) {
            let secs = wiki::sections(&page.body);
            for (ln, line) in &added {
                let t = line.trim_start();
                if !(t.starts_with("- ") || t.starts_with("* ")) || t.contains(" ^c-") || is_link_only(t) {
                    continue;
                }
                let sec = secs.iter().filter(|s| s.start <= *ln && *ln <= s.end).max_by_key(|s| s.level).map(|s| normalize(&s.heading));
                if sec.as_deref().is_some_and(|h| NON_CLAIM_SECTIONS.iter().any(|n| normalize(n) == h)) {
                    continue;
                }
                r.errors.push(format!("{path}:{ln}: statements about the user in health/mind/profile pages must be claims (use claim_propose)."));
            }
        }

        // Human-written text may not be removed or rewritten (§5.5, §6.4).
        if !old.is_empty() {
            let human = human_lines(path);
            if !human.is_empty() {
                let old_lines: Vec<&str> = old.lines().collect();
                // Frontmatter is structured data managed through the tools; only body text is protected.
                let fm_end = if old_lines.first().map(|l| l.trim()) == Some("---") {
                    old_lines.iter().skip(1).position(|l| l.trim() == "---").map(|p| p + 2).unwrap_or(0)
                } else {
                    0
                };
                for ln in removed_lines(&old, new) {
                    if ln > fm_end && human.contains(&ln) && old_lines.get(ln - 1).is_some_and(|l| !l.trim().is_empty()) {
                        r.errors.push(format!("{path}: line {ln} was written by the user and may not be changed without their approval."));
                    }
                }
            }
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn added_and_removed_are_multiset_diffs() {
        assert_eq!(added_lines("a\nb\n", "a\nb\nc\nb\n"), vec![(3, "c"), (4, "b")]);
        assert_eq!(removed_lines("a\nb\nc", "a\nc"), vec![2]);
    }

    #[test]
    fn link_only_lines() {
        assert!(is_link_only("- [[sara|Sara]], [[ali]]"));
        assert!(!is_link_only("- met [[sara|Sara]]"));
    }
}
