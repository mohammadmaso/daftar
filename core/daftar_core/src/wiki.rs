//! Wiki pages (§3.3): frontmatter model, sections, wikilinks and claims (§3.4).
//!
//! Frontmatter is parsed leniently (unknown properties added in Obsidian are preserved and
//! re-emitted after the known ones) and rendered canonically so diffs stay small.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use crate::config::Bilingual;
use crate::frontmatter::{self, quote};
use crate::{Error, Result, ids};

pub const PAGE_TYPES: &[&str] = &[
    "person",
    "topic",
    "concern",
    "condition",
    "medication",
    "lab",
    "journal-day",
    "profile",
    "pattern",
    "idea",
    "character",
    "place",
    "thread",
    "answer",
    "summary",
    "review",
    "project",
    "goal",
    "visit",
    "symptom-log",
    "chapter",
    "draft",
    "story",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageMeta {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub vault: String,
    #[serde(default = "empty_title")]
    pub title: Bilingual,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub updated: String,
    #[serde(default = "active")]
    pub status: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

fn empty_title() -> Bilingual {
    Bilingual {
        en: String::new(),
        fa: String::new(),
    }
}
fn active() -> String {
    "active".into()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub path: String,
    pub meta: PageMeta,
    pub body: String,
}

impl Page {
    pub fn slug(&self) -> &str {
        slug_of(&self.path)
    }

    pub fn render(&self) -> String {
        render(&self.meta, &self.body)
    }

    /// Display title in the given UI language, falling back to the other language, then the slug.
    pub fn title(&self, lang: &str) -> &str {
        let (a, b) = if lang == "fa" {
            (&self.meta.title.fa, &self.meta.title.en)
        } else {
            (&self.meta.title.en, &self.meta.title.fa)
        };
        if !a.is_empty() {
            a
        } else if !b.is_empty() {
            b
        } else {
            self.slug()
        }
    }
}

pub fn slug_of(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".md").unwrap_or(name)
}

/// Normalises a model-proposed slug to ASCII kebab-case (NFC first so composed forms are stable).
pub fn sanitize_slug(proposed: &str) -> String {
    let nfc: String = proposed.nfc().collect();
    let s = ids::kebab_slug(&nfc);
    let mut s: String = s.chars().take(80).collect();
    while s.ends_with('-') {
        s.pop();
    }
    if s.is_empty() {
        format!(
            "page-{}",
            ids::new_id().to_string()[16..].to_ascii_lowercase()
        )
    } else {
        s
    }
}

pub fn parse(path: &str, doc: &str) -> Result<Page> {
    let (yaml, body) = frontmatter::split(doc);
    let meta: PageMeta = match yaml {
        Some(y) if !y.trim().is_empty() => serde_saphyr::from_str(y)
            .map_err(|e| Error::invalid(format!("{path}: frontmatter: {e}")))?,
        _ => PageMeta {
            id: String::new(),
            kind: String::new(),
            vault: String::new(),
            title: empty_title(),
            aliases: vec![],
            summary: String::new(),
            sources: vec![],
            created: String::new(),
            updated: String::new(),
            status: active(),
            extra: BTreeMap::new(),
        },
    };
    Ok(Page {
        path: path.to_owned(),
        meta,
        body: body.trim_start_matches(['\n', '\r']).to_owned(),
    })
}

fn yaml_scalar(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => quote(s),
        Value::Array(a) => format!(
            "[{}]",
            a.iter().map(yaml_scalar).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(o) => format!(
            "{{ {} }}",
            o.iter()
                .map(|(k, v)| format!("{k}: {}", yaml_scalar(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn list(v: &[String]) -> String {
    format!(
        "[{}]",
        v.iter().map(|s| quote(s)).collect::<Vec<_>>().join(", ")
    )
}

pub fn render(m: &PageMeta, body: &str) -> String {
    let mut out = String::from("---\n");
    out.push_str(&format!("id: {}\n", m.id));
    out.push_str(&format!("type: {}\n", m.kind));
    out.push_str(&format!("vault: {}\n", m.vault));
    out.push_str(&format!(
        "title: {{ en: {}, fa: {} }}\n",
        quote(&m.title.en),
        quote(&m.title.fa)
    ));
    out.push_str(&format!("aliases: {}\n", list(&m.aliases)));
    out.push_str(&format!("summary: {}\n", quote(&m.summary)));
    out.push_str(&format!("sources: {}\n", list(&m.sources)));
    out.push_str(&format!("created: {}\n", m.created));
    out.push_str(&format!("updated: {}\n", m.updated));
    out.push_str(&format!("status: {}\n", m.status));
    for (k, v) in &m.extra {
        out.push_str(&format!("{k}: {}\n", yaml_scalar(v)));
    }
    out.push_str("---\n\n");
    out.push_str(body.trim_start_matches('\n').trim_end());
    out.push('\n');
    out
}

// ─────────────────────────── text helpers ───────────────────────────

/// Short content hash used as `base_hash` for optimistic edits.
pub fn content_hash(text: &str) -> String {
    let d = Sha256::digest(text.as_bytes());
    d.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

/// `  12 | text` numbering (1-based) as shown to the model.
pub fn line_numbered(text: &str, from: usize, to: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let to = to.min(lines.len());
    let width = to.to_string().len();
    (from.max(1)..=to)
        .map(|n| format!("{n:>width$} | {}", lines[n - 1]))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Section {
    pub heading: String,
    pub level: usize,
    /// 1-based inclusive line range of the section in the document (heading line included).
    pub start: usize,
    pub end: usize,
}

/// Markdown sections by ATX heading, ignoring headings inside fenced code.
pub fn sections(text: &str) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    let mut fence = false;
    let lines: Vec<&str> = text.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = !fence;
            continue;
        }
        if fence {
            continue;
        }
        let level = t.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&level) && t[level..].starts_with(' ') {
            if let Some(prev) = out.last_mut() {
                prev.end = i;
            }
            out.push(Section {
                heading: t[level..].trim().to_owned(),
                level,
                start: i + 1,
                end: lines.len(),
            });
        }
    }
    // A section ends where the next heading of the same or higher level starts.
    for i in 0..out.len() {
        let lvl = out[i].level;
        let next = out[i + 1..]
            .iter()
            .find(|s| s.level <= lvl)
            .map(|s| s.start - 1)
            .unwrap_or(lines.len());
        out[i].end = next;
    }
    out
}

// ─────────────────────────── wikilinks ───────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Link {
    pub target: String,
    pub fragment: Option<String>,
    pub label: Option<String>,
    pub embed: bool,
    /// Byte range in the source text.
    pub start: usize,
    pub end: usize,
}

pub fn links(text: &str) -> Vec<Link> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut in_code = false;
    while i + 1 < bytes.len() {
        if bytes[i] == b'`' {
            in_code = !in_code;
            i += 1;
            continue;
        }
        if !in_code && bytes[i] == b'[' && bytes[i + 1] == b'[' {
            let embed = i > 0 && bytes[i - 1] == b'!';
            if let Some(close) = text[i + 2..].find("]]") {
                let inner = &text[i + 2..i + 2 + close];
                if !inner.contains('\n') && !inner.is_empty() {
                    let (dest, label) = match inner.split_once('|') {
                        Some((d, l)) => (d, Some(l.trim().to_owned())),
                        None => (inner, None),
                    };
                    let (target, fragment) = match dest.find(['#', '^']) {
                        Some(p) => (&dest[..p], Some(dest[p..].to_owned())),
                        None => (dest, None),
                    };
                    out.push(Link {
                        target: target.trim().trim_end_matches(".md").to_owned(),
                        fragment,
                        label,
                        embed,
                        start: if embed { i - 1 } else { i },
                        end: i + 2 + close + 2,
                    });
                    i = i + 2 + close + 2;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

// ─────────────────────────── claims ───────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub text: String,
    pub status: String,
    pub confidence: Option<String>,
    pub sources: Vec<String>,
    /// 1-based line number.
    pub line: usize,
}

/// Parses `- text (status:: x) (confidence:: y) (src:: [[..]], [[..]]) ^c-ID` list items.
pub fn claims(text: &str) -> Vec<Claim> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let t = line.trim_start();
        let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) else {
            continue;
        };
        let Some(caret) = rest.rfind(" ^c-") else {
            continue;
        };
        let id = rest[caret + 2..].trim().to_owned();
        let body = &rest[..caret];
        let mut fields = BTreeMap::new();
        let mut plain = String::new();
        let mut j = 0;
        let b = body.as_bytes();
        while j < b.len() {
            if b[j] == b'('
                && let Some(close) = matching_paren(body, j)
            {
                let inner = &body[j + 1..close];
                if let Some((k, v)) = inner.split_once("::") {
                    fields.insert(k.trim().to_owned(), v.trim().to_owned());
                    j = close + 1;
                    continue;
                }
            }
            let ch = body[j..].chars().next().expect("in bounds");
            plain.push(ch);
            j += ch.len_utf8();
        }
        let sources = fields
            .get("src")
            .map(|s| links(s).into_iter().map(|l| l.target).collect())
            .unwrap_or_default();
        out.push(Claim {
            id,
            text: plain.split_whitespace().collect::<Vec<_>>().join(" "),
            status: fields
                .get("status")
                .cloned()
                .unwrap_or_else(|| "confirmed".into()),
            confidence: fields.get("confidence").cloned(),
            sources,
            line: i + 1,
        });
    }
    out
}

fn matching_paren(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (k, c) in s[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + k);
                }
            }
            _ => {}
        }
    }
    None
}

/// Formats a claim line (§3.4). `sources` are `(raw_path, label)` pairs.
pub fn claim_line(
    text: &str,
    status: &str,
    confidence: Option<&str>,
    sources: &[(String, String)],
    id: &str,
) -> String {
    let src = sources
        .iter()
        .map(|(p, l)| format!("[[{}|{}]]", p.trim_end_matches(".md"), l))
        .collect::<Vec<_>>()
        .join(", ");
    let conf = confidence
        .map(|c| format!(" (confidence:: {c})"))
        .unwrap_or_default();
    format!(
        "- {} (status:: {status}){conf} (src:: {src}) ^{id}",
        text.trim().trim_end_matches('.')
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "---\nid: 01JAB\ntype: person\nvault: life\ntitle: { en: \"Sara\", fa: \"سارا\" }\naliases: [\"sara\", \"سارا\"]\nsummary: \"My cousin.\"\nsources: [\"01JABC\"]\ncreated: 2026-09-23\nupdated: 2026-09-23\nstatus: active\ncssclasses: [wide]\n---\n\n# Sara\n\n## Who\nMy cousin, lives in [[shiraz|Shiraz]].\n\n## Timeline\n- 23 Sep: called about [[isfahan-trip]] ([[raw/2026/09/23/x|voice · 23 Sep]])\n";

    #[test]
    fn frontmatter_round_trip_keeps_unknown_properties() {
        let p = parse("vaults/life/people/sara.md", DOC).unwrap();
        assert_eq!(p.meta.title.fa, "سارا");
        assert_eq!(p.meta.extra["cssclasses"], serde_json::json!(["wide"]));
        let again = parse(&p.path, &p.render()).unwrap();
        assert_eq!(again, p);
        assert_eq!(p.slug(), "sara");
        assert_eq!(p.title("fa"), "سارا");
    }

    #[test]
    fn links_and_sections() {
        let p = parse("x.md", DOC).unwrap();
        let ls = links(&p.body);
        let targets: Vec<_> = ls.iter().map(|l| l.target.as_str()).collect();
        assert_eq!(targets, vec!["shiraz", "isfahan-trip", "raw/2026/09/23/x"]);
        assert_eq!(ls[0].label.as_deref(), Some("Shiraz"));
        let s = sections(&p.body);
        assert_eq!(
            s.iter().map(|s| s.heading.as_str()).collect::<Vec<_>>(),
            vec!["Sara", "Who", "Timeline"]
        );
        assert_eq!(s[0].end, p.body.lines().count(), "h1 spans the whole page");
        assert_eq!((s[1].start, s[1].end), (3, 5));
        assert!(
            links("`[[not a link]]` and [[real]]")
                .iter()
                .map(|l| &l.target)
                .eq(["real"].iter())
        );
    }

    #[test]
    fn claims_parse_and_format() {
        let line = claim_line(
            "Takes vitamin D 50,000 IU weekly.",
            "confirmed",
            None,
            &[("raw/2026/03/03/x.md".into(), "voice · 3 Mar".into())],
            "c-01JAB9",
        );
        assert_eq!(
            line,
            "- Takes vitamin D 50,000 IU weekly (status:: confirmed) (src:: [[raw/2026/03/03/x|voice · 3 Mar]]) ^c-01JAB9"
        );
        let doc = format!(
            "## Claims\n{line}\n- Sleeps worse after late coffee (status:: proposed) (confidence:: medium) (src:: [[raw/a]], [[raw/b]]) ^c-01JAC2\n- plain item\n"
        );
        let cs = claims(&doc);
        assert_eq!(cs.len(), 2);
        assert_eq!(cs[0].text, "Takes vitamin D 50,000 IU weekly");
        assert_eq!(cs[0].sources, vec!["raw/2026/03/03/x"]);
        assert_eq!(cs[1].status, "proposed");
        assert_eq!(cs[1].confidence.as_deref(), Some("medium"));
        assert_eq!(cs[1].line, 3);
    }

    #[test]
    fn slugs_and_hashes() {
        assert_eq!(
            sanitize_slug("Vitamin D Deficiency"),
            "vitamin-d-deficiency"
        );
        assert!(sanitize_slug("کمبود ویتامین").starts_with("page-"));
        assert_eq!(content_hash("a"), content_hash("a"));
        assert_ne!(content_hash("a"), content_hash("b"));
        assert_eq!(line_numbered("a\nb\nc", 2, 3), "2 | b\n3 | c");
    }
}
