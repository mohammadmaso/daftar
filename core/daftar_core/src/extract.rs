//! File import (§4.1 "share-sheet import"): turns a picked or dropped file into plain text the user
//! previews before it becomes a raw `import` capture. Nothing here touches the library; the
//! original file is never committed (binaries would bloat history, like audio in §3.1), only the
//! extracted text is.
//!
//! Supported: any text file in any common encoding (UTF-8/16, legacy code pages such as
//! Windows-1256 for Persian), Markdown, HTML/XHTML, RTF, DOCX, ODT, PPTX, EPUB and PDF with a text
//! layer. Audio files are recognised so the caller can hand them to the voice pipeline instead.

use std::io::{Cursor, Read};
use std::path::Path;

use encoding_rs::Encoding;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use unicode_normalization::UnicodeNormalization;

use crate::{Error, Result};

/// Largest document read at all.
pub const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;
/// Largest audio file: the OpenAI-compatible transcription limit.
pub const MAX_AUDIO_BYTES: u64 = 25 * 1024 * 1024;
/// Text beyond this is not filed: one capture has to fit an ingest model's context comfortably.
pub const MAX_IMPORT_CHARS: usize = 60_000;

/// Audio containers the speech-to-text endpoints accept. `.opus` is Ogg and stored as such.
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "m4a", "mp4", "mpeg", "mpga", "wav", "ogg", "oga", "opus", "flac", "webm",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    Text,
    Markdown,
    Html,
    Rtf,
    Docx,
    Odt,
    Pptx,
    Epub,
    Pdf,
}

impl DocKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Markdown => "markdown",
            Self::Html => "html",
            Self::Rtf => "rtf",
            Self::Docx => "docx",
            Self::Odt => "odt",
            Self::Pptx => "pptx",
            Self::Epub => "epub",
            Self::Pdf => "pdf",
        }
    }
}

/// What a file is, decided from its name and (for text) its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileClass {
    Document(DocKind),
    Audio,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Extracted {
    pub kind: DocKind,
    /// The file name as the user saw it.
    pub name: String,
    /// The text that will be filed (already cut to [`MAX_IMPORT_CHARS`]).
    pub text: String,
    /// Characters in the whole document before cutting.
    pub total_chars: usize,
    pub truncated: bool,
    /// Pages (PDF) or slides (PPTX), when the format has them.
    pub pages: Option<u32>,
    /// The detected encoding of a plain-text file, when it wasn't UTF-8.
    pub encoding: Option<String>,
}

fn ext_of(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Classifies by extension. Unknown extensions are treated as text and checked when read.
pub fn classify(name: &str) -> FileClass {
    let ext = ext_of(name);
    if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
        return FileClass::Audio;
    }
    FileClass::Document(match ext.as_str() {
        "md" | "markdown" | "mdown" | "mkd" => DocKind::Markdown,
        "html" | "htm" | "xhtml" => DocKind::Html,
        "rtf" => DocKind::Rtf,
        "docx" | "docm" | "dotx" => DocKind::Docx,
        "odt" | "ott" => DocKind::Odt,
        "pptx" => DocKind::Pptx,
        "epub" => DocKind::Epub,
        "pdf" => DocKind::Pdf,
        _ => DocKind::Text,
    })
}

/// The extension an imported recording is stored under (the transcriber reads the format from it).
pub fn audio_store_ext(name: &str) -> String {
    match ext_of(name).as_str() {
        "opus" | "oga" => "ogg".into(),
        e => e.into(),
    }
}

/// Reads and extracts a file from disk.
pub fn extract_path(path: &Path) -> Result<Extracted> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());
    let len = std::fs::metadata(path)?.len();
    if len > MAX_FILE_BYTES {
        return Err(Error::invalid(
            "This file is too large to import (over 50 MB).",
        ));
    }
    let bytes = std::fs::read(path)?;
    extract(&name, &bytes)
}

/// Extracts `bytes`, named `name`, into text.
pub fn extract(name: &str, bytes: &[u8]) -> Result<Extracted> {
    let kind = match classify(name) {
        FileClass::Audio => {
            return Err(Error::invalid(
                "This is a recording; it is transcribed, not read.",
            ));
        }
        FileClass::Document(k) => k,
    };
    // A .txt that is really a PDF or an Office file still reads correctly.
    let kind = match (kind, sniff(bytes)) {
        (DocKind::Text | DocKind::Markdown, Some(k)) => k,
        (k, _) => k,
    };
    let mut pages = None;
    let mut encoding = None;
    let raw = match kind {
        DocKind::Text | DocKind::Markdown => {
            let (t, enc) = decode_text(bytes)?;
            encoding = enc;
            t
        }
        DocKind::Html => html_to_text(&decode_text(bytes)?.0),
        DocKind::Rtf => rtf_to_text(bytes),
        DocKind::Docx => docx_to_text(bytes)?,
        DocKind::Odt => odt_to_text(bytes)?,
        DocKind::Pptx => {
            let (t, n) = pptx_to_text(bytes)?;
            pages = Some(n);
            t
        }
        DocKind::Epub => epub_to_text(bytes)?,
        DocKind::Pdf => {
            let (t, n) = pdf_to_text(bytes)?;
            pages = Some(n);
            t
        }
    };
    let text = tidy(&raw);
    if text.trim().is_empty() {
        return Err(Error::invalid(if kind == DocKind::Pdf {
            "This PDF has no text in it; scanned pages can be added as photos."
        } else {
            "This file has no text in it."
        }));
    }
    let total_chars = text.chars().count();
    let (text, truncated) = cut(&text, MAX_IMPORT_CHARS);
    Ok(Extracted {
        kind,
        name: name.to_owned(),
        text,
        total_chars,
        truncated,
        pages,
        encoding,
    })
}

fn sniff(bytes: &[u8]) -> Option<DocKind> {
    if bytes.starts_with(b"%PDF-") {
        return Some(DocKind::Pdf);
    }
    if bytes.starts_with(b"PK\x03\x04") {
        let mut z = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
        if z.by_name("word/document.xml").is_ok() {
            return Some(DocKind::Docx);
        }
        if z.by_name("ppt/presentation.xml").is_ok() {
            return Some(DocKind::Pptx);
        }
        if z.by_name("META-INF/container.xml").is_ok() {
            return Some(DocKind::Epub);
        }
        if z.by_name("content.xml").is_ok() {
            return Some(DocKind::Odt);
        }
    }
    if bytes.starts_with(b"{\\rtf") {
        return Some(DocKind::Rtf);
    }
    None
}

// ─────────────────────────── plain text ───────────────────────────

/// Decodes text in whatever encoding it is in. Returns the encoding name when it wasn't UTF-8.
/// Binary files (NUL bytes outside UTF-16) are refused rather than shown as noise.
pub fn decode_text(bytes: &[u8]) -> Result<(String, Option<String>)> {
    if let Some((enc, bom)) = Encoding::for_bom(bytes) {
        let (t, _) = enc.decode_without_bom_handling(&bytes[bom..]);
        let name = (enc != encoding_rs::UTF_8).then(|| enc.name().to_owned());
        return Ok((t.into_owned(), name));
    }
    let head = &bytes[..bytes.len().min(8192)];
    if head.contains(&0) {
        return Err(Error::invalid(
            "This file isn't text, so it can't be imported.",
        ));
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return Ok((s.to_owned(), None));
    }
    let mut det = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Deny);
    det.feed(bytes, true);
    let enc = det.guess(None, chardetng::Utf8Detection::Deny);
    let (t, _, _) = enc.decode(bytes);
    Ok((t.into_owned(), Some(enc.name().to_owned())))
}

/// Normalises line endings, trailing spaces and runs of blank lines; folds Arabic presentation
/// forms (common in PDFs) back to ordinary letters so search and the model see real Persian.
fn tidy(s: &str) -> String {
    let s = s.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::with_capacity(s.len());
    let mut blank = 0;
    for line in s.lines() {
        let line: String = if line.chars().any(|c| {
            ('\u{FB50}'..='\u{FDFF}').contains(&c) || ('\u{FE70}'..='\u{FEFF}').contains(&c)
        }) {
            line.chars()
                .filter(|&c| c != '\u{FEFF}')
                .flat_map(|c| {
                    if ('\u{FB50}'..='\u{FDFF}').contains(&c)
                        || ('\u{FE70}'..='\u{FEFE}').contains(&c)
                    {
                        c.nfkc().collect::<Vec<_>>()
                    } else {
                        vec![c]
                    }
                })
                .collect()
        } else {
            line.replace('\u{FEFF}', "")
        };
        let line = line.trim_end();
        if line.is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_owned()
}

/// Cuts at a paragraph (or line) boundary at or before `max` characters.
fn cut(s: &str, max: usize) -> (String, bool) {
    let Some((byte, _)) = s.char_indices().nth(max) else {
        return (s.to_owned(), false);
    };
    let head = &s[..byte];
    let at = head
        .rfind("\n\n")
        .filter(|&i| i > byte / 2)
        .or_else(|| head.rfind('\n').filter(|&i| i > byte / 2))
        .unwrap_or(byte);
    (head[..at].trim_end().to_owned(), true)
}

// ─────────────────────────── HTML ───────────────────────────

/// A tolerant HTML → text pass: block elements become line breaks, headings and list items keep a
/// Markdown marker, script/style/head are dropped and entities decoded.
pub fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut rest = html;
    let mut skip_until: Option<&'static str> = None;
    while let Some(lt) = rest.find('<') {
        if skip_until.is_none() {
            out.push_str(&decode_entities(&collapse_ws(&rest[..lt])));
        }
        rest = &rest[lt..];
        if rest.starts_with("<!--") {
            rest = rest.find("-->").map_or("", |i| &rest[i + 3..]);
            continue;
        }
        let Some(gt) = rest.find('>') else {
            rest = "";
            break;
        };
        let tag = &rest[1..gt];
        rest = &rest[gt + 1..];
        let closing = tag.starts_with('/');
        let name: String = tag
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        if let Some(end) = skip_until {
            if closing && name == end {
                skip_until = None;
            }
            continue;
        }
        match name.as_str() {
            "script" | "style" | "head" | "noscript" | "template" | "svg" if !closing => {
                if !tag.ends_with('/') {
                    skip_until = Some(match name.as_str() {
                        "script" => "script",
                        "style" => "style",
                        "head" => "head",
                        "noscript" => "noscript",
                        "template" => "template",
                        _ => "svg",
                    });
                }
            }
            "br" => out.push('\n'),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                out.push_str("\n\n");
                if !closing {
                    let level = name[1..].parse::<usize>().unwrap_or(1);
                    out.push_str(&"#".repeat(level));
                    out.push(' ');
                }
            }
            "li" if !closing => out.push_str("\n- "),
            "p" | "div" | "section" | "article" | "blockquote" | "pre" | "table" | "ul" | "ol"
            | "header" | "footer" | "main" | "nav" | "aside" | "figure" | "hr" => {
                out.push_str("\n\n")
            }
            "tr" => out.push('\n'),
            "td" | "th" if !closing => out.push_str(" | "),
            _ => {}
        }
    }
    if skip_until.is_none() {
        out.push_str(&decode_entities(&collapse_ws(rest)));
    }
    // Lines of a list or table start with the marker we inserted, not with spaces.
    out.lines()
        .map(|l| l.trim().trim_start_matches("| ").to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !space {
                out.push(' ');
            }
            space = true;
        } else {
            out.push(c);
            space = false;
        }
    }
    out
}

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(semi) = rest[..rest.len().min(12)].find(';') else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..semi];
        let ch = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            "zwnj" => Some('\u{200C}'),
            "zwj" => Some('\u{200D}'),
            "rlm" => Some('\u{200F}'),
            "lrm" => Some('\u{200E}'),
            "mdash" => Some('—'),
            "ndash" => Some('–'),
            "hellip" => Some('…'),
            "laquo" => Some('«'),
            "raquo" => Some('»'),
            e if e.starts_with("#x") || e.starts_with("#X") => u32::from_str_radix(&e[2..], 16)
                .ok()
                .and_then(char::from_u32),
            e if e.starts_with('#') => e[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

// ─────────────────────────── RTF ───────────────────────────

/// RTF → text: paragraphs, tabs, `\'hh` bytes in the document's code page and `\uN` characters.
/// Font tables, styles, pictures and other destinations are skipped.
pub fn rtf_to_text(bytes: &[u8]) -> String {
    let mut out = String::new();
    // Per group: (skipping, unicode fallback count)
    let mut stack: Vec<(bool, usize)> = vec![(false, 1)];
    let mut enc: &'static Encoding = encoding_rs::WINDOWS_1252;
    let mut pending: Vec<u8> = Vec::new();
    let mut skip_fallback = 0usize;
    let mut i = 0;
    let flush = |pending: &mut Vec<u8>, out: &mut String, enc: &'static Encoding| {
        if !pending.is_empty() {
            out.push_str(&enc.decode_without_bom_handling(pending).0);
            pending.clear();
        }
    };
    while i < bytes.len() {
        let c = bytes[i];
        let skipping = stack.last().is_some_and(|s| s.0);
        match c {
            b'{' => {
                flush(&mut pending, &mut out, enc);
                let top = *stack.last().unwrap_or(&(false, 1));
                stack.push(top);
                if bytes[i + 1..].starts_with(b"\\*")
                    && let Some(t) = stack.last_mut()
                {
                    t.0 = true;
                }
                i += 1;
            }
            b'}' => {
                flush(&mut pending, &mut out, enc);
                stack.pop();
                if stack.is_empty() {
                    break;
                }
                i += 1;
            }
            b'\\' => {
                i += 1;
                let Some(&n) = bytes.get(i) else { break };
                if n == b'\'' {
                    let hex = bytes.get(i + 1..i + 3).unwrap_or_default();
                    i += 3;
                    if skip_fallback > 0 {
                        skip_fallback -= 1;
                        continue;
                    }
                    if !skipping
                        && let Ok(b) =
                            u8::from_str_radix(std::str::from_utf8(hex).unwrap_or("x"), 16)
                    {
                        pending.push(b);
                    }
                    continue;
                }
                if !n.is_ascii_alphabetic() {
                    // Control symbols: \\ \{ \} \~ \- \_ and line breaks.
                    i += 1;
                    if skipping {
                        continue;
                    }
                    flush(&mut pending, &mut out, enc);
                    match n {
                        b'\\' | b'{' | b'}' => out.push(n as char),
                        b'~' => out.push('\u{00A0}'),
                        b'_' => out.push('-'),
                        b'\n' | b'\r' => out.push('\n'),
                        _ => {}
                    }
                    continue;
                }
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let word = std::str::from_utf8(&bytes[start..i]).unwrap_or_default();
                let num_start = i;
                if i < bytes.len() && (bytes[i] == b'-' || bytes[i].is_ascii_digit()) {
                    i += 1;
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let num: Option<i32> = std::str::from_utf8(&bytes[num_start..i])
                    .ok()
                    .and_then(|s| s.parse().ok());
                if bytes.get(i) == Some(&b' ') {
                    i += 1;
                }
                match word {
                    "fonttbl" | "colortbl" | "stylesheet" | "info" | "pict" | "object"
                    | "header" | "footer" | "headerl" | "headerr" | "footerl" | "footerr"
                    | "listtable" | "listoverridetable" | "rsidtbl" | "themedata" | "datastore"
                    | "latentstyles" | "generator" | "xmlnstbl" => {
                        if let Some(t) = stack.last_mut() {
                            t.0 = true;
                        }
                    }
                    "ansicpg" => {
                        if let Some(e) = num.and_then(|n| codepage(n as u16)) {
                            enc = e;
                        }
                    }
                    "uc" => {
                        if let Some(t) = stack.last_mut() {
                            t.1 = num.unwrap_or(1).max(0) as usize;
                        }
                    }
                    "u" if !skipping => {
                        flush(&mut pending, &mut out, enc);
                        let v = num.unwrap_or(0);
                        let v = if v < 0 { v + 65536 } else { v } as u32;
                        if let Some(ch) = char::from_u32(v) {
                            out.push(ch);
                        }
                        skip_fallback = stack.last().map_or(1, |s| s.1);
                    }
                    "par" | "line" | "row" | "sect" | "page" if !skipping => {
                        flush(&mut pending, &mut out, enc);
                        out.push('\n');
                    }
                    "tab" | "cell" if !skipping => {
                        flush(&mut pending, &mut out, enc);
                        out.push('\t');
                    }
                    _ => {}
                }
            }
            b'\r' | b'\n' => i += 1,
            _ => {
                i += 1;
                if skip_fallback > 0 {
                    skip_fallback -= 1;
                    continue;
                }
                if !skipping {
                    pending.push(c);
                }
            }
        }
    }
    flush(&mut pending, &mut out, enc);
    out
}

fn codepage(cp: u16) -> Option<&'static Encoding> {
    Some(match cp {
        1250 => encoding_rs::WINDOWS_1250,
        1251 => encoding_rs::WINDOWS_1251,
        1252 => encoding_rs::WINDOWS_1252,
        1253 => encoding_rs::WINDOWS_1253,
        1254 => encoding_rs::WINDOWS_1254,
        1255 => encoding_rs::WINDOWS_1255,
        1256 => encoding_rs::WINDOWS_1256,
        1257 => encoding_rs::WINDOWS_1257,
        1258 => encoding_rs::WINDOWS_1258,
        65001 => encoding_rs::UTF_8,
        _ => return None,
    })
}

// ─────────────────────────── zip + XML formats ───────────────────────────

fn open_zip(bytes: &[u8]) -> Result<zip::ZipArchive<Cursor<&[u8]>>> {
    zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| Error::invalid("This file is damaged and can't be read."))
}

fn zip_entry(z: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<String> {
    let mut f = z
        .by_name(name)
        .map_err(|_| Error::invalid("This file is damaged and can't be read."))?;
    let mut s = String::new();
    f.read_to_string(&mut s)
        .map_err(|_| Error::invalid("This file is damaged and can't be read."))?;
    Ok(s)
}

fn local(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, l)| l)
}

fn attr(e: &quick_xml::events::BytesStart<'_>, key: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| local(a.key.as_ref()) == key)
        .and_then(|a| {
            a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
                .map(|v| v.into_owned())
        })
}

fn xml_err(_: impl std::fmt::Debug) -> Error {
    Error::invalid("This file is damaged and can't be read.")
}

/// DOCX body: paragraphs, headings (Heading1…6/Title styles), list items and table cells.
pub fn docx_to_text(bytes: &[u8]) -> Result<String> {
    let mut z = open_zip(bytes)?;
    let xml = zip_entry(&mut z, "word/document.xml")?;
    let mut r = Reader::from_str(&xml);
    let mut out = String::new();
    let mut para = String::new();
    let mut prefix = String::new();
    let mut in_text = false;
    loop {
        match r.read_event().map_err(xml_err)? {
            Event::Start(e) | Event::Empty(e) => match local(e.name().as_ref()) {
                "t" => in_text = true,
                "tab" => para.push('\t'),
                "br" | "cr" => para.push('\n'),
                "pStyle" => {
                    let v = attr(&e, "val").unwrap_or_default().to_ascii_lowercase();
                    if v == "title" {
                        prefix = "# ".into();
                    } else if let Some(n) = v
                        .strip_prefix("heading")
                        .and_then(|n| n.trim().parse::<usize>().ok())
                    {
                        prefix = format!("{} ", "#".repeat(n.clamp(1, 6)));
                    }
                }
                "numPr" if prefix.is_empty() => prefix = "- ".into(),
                "tc" => para.push_str(" | "),
                _ => {}
            },
            Event::Text(t) if in_text => {
                para.push_str(&t.xml10_content());
            }
            Event::GeneralRef(g) if in_text => {
                para.push_str(&decode_entities(&format!("&{};", g.xml10_content())));
            }
            Event::End(e) => match local(e.name().as_ref()) {
                "t" => in_text = false,
                "p" => {
                    let p = para.trim();
                    if !p.is_empty() {
                        out.push_str(&prefix);
                        out.push_str(p.trim_start_matches("| "));
                    }
                    out.push_str("\n\n");
                    para.clear();
                    prefix.clear();
                }
                "tr" => out.push('\n'),
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(out)
}

/// ODT body: `text:h` headings with their outline level, paragraphs, list items, spaces and tabs.
pub fn odt_to_text(bytes: &[u8]) -> Result<String> {
    let mut z = open_zip(bytes)?;
    let xml = zip_entry(&mut z, "content.xml")?;
    let mut r = Reader::from_str(&xml);
    let mut out = String::new();
    let mut in_body = false;
    let mut depth_list = 0usize;
    loop {
        match r.read_event().map_err(xml_err)? {
            Event::Start(e) => match local(e.name().as_ref()) {
                "body" => in_body = true,
                "h" => {
                    let level = attr(&e, "outline-level")
                        .and_then(|l| l.parse::<usize>().ok())
                        .unwrap_or(1);
                    out.push_str(&format!("\n\n{} ", "#".repeat(level.clamp(1, 6))));
                }
                "p" if depth_list > 0 => out.push_str("\n- "),
                "p" => out.push_str("\n\n"),
                "list" => depth_list += 1,
                "table-row" => out.push('\n'),
                "table-cell" => out.push_str(" | "),
                _ => {}
            },
            Event::Empty(e) => match local(e.name().as_ref()) {
                "s" => {
                    let n = attr(&e, "c").and_then(|c| c.parse().ok()).unwrap_or(1);
                    out.push_str(&" ".repeat(n));
                }
                "tab" => out.push('\t'),
                "line-break" => out.push('\n'),
                _ => {}
            },
            Event::End(e) => {
                if local(e.name().as_ref()) == "list" {
                    depth_list = depth_list.saturating_sub(1);
                }
            }
            Event::Text(t) if in_body => out.push_str(&t.xml10_content()),
            Event::GeneralRef(g) if in_body => {
                out.push_str(&decode_entities(&format!("&{};", g.xml10_content())))
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(out
        .lines()
        .map(|l| l.trim_start_matches(" | "))
        .collect::<Vec<_>>()
        .join("\n"))
}

/// PPTX: every slide in order, each under a "Slide n" heading; one line per paragraph.
pub fn pptx_to_text(bytes: &[u8]) -> Result<(String, u32)> {
    let mut z = open_zip(bytes)?;
    let mut slides: Vec<(u32, String)> = z
        .file_names()
        .filter_map(|n| {
            let num = n
                .strip_prefix("ppt/slides/slide")?
                .strip_suffix(".xml")?
                .parse()
                .ok()?;
            Some((num, n.to_owned()))
        })
        .collect();
    slides.sort();
    let mut out = String::new();
    for (i, (_, name)) in slides.iter().enumerate() {
        let xml = zip_entry(&mut z, name)?;
        let mut r = Reader::from_str(&xml);
        out.push_str(&format!("## Slide {}\n\n", i + 1));
        let mut in_text = false;
        loop {
            match r.read_event().map_err(xml_err)? {
                Event::Start(e) if local(e.name().as_ref()) == "t" => in_text = true,
                Event::End(e) => match local(e.name().as_ref()) {
                    "t" => in_text = false,
                    "p" => out.push('\n'),
                    _ => {}
                },
                Event::Text(t) if in_text => out.push_str(&t.xml10_content()),
                Event::GeneralRef(g) if in_text => {
                    out.push_str(&decode_entities(&format!("&{};", g.xml10_content())))
                }
                Event::Eof => break,
                _ => {}
            }
        }
        out.push('\n');
    }
    Ok((out, slides.len() as u32))
}

/// EPUB: the spine's documents in reading order, each converted as HTML.
pub fn epub_to_text(bytes: &[u8]) -> Result<String> {
    let mut z = open_zip(bytes)?;
    let container = zip_entry(&mut z, "META-INF/container.xml")?;
    let mut r = Reader::from_str(&container);
    let mut opf_path = None;
    loop {
        match r.read_event().map_err(xml_err)? {
            Event::Start(e) | Event::Empty(e) if local(e.name().as_ref()) == "rootfile" => {
                opf_path = attr(&e, "full-path");
                break;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    let opf_path = opf_path.ok_or_else(|| xml_err(()))?;
    let base = opf_path.rsplit_once('/').map_or("", |(d, _)| d).to_owned();
    let opf = zip_entry(&mut z, &opf_path)?;
    let mut manifest = std::collections::HashMap::new();
    let mut spine = Vec::new();
    let mut r = Reader::from_str(&opf);
    loop {
        match r.read_event().map_err(xml_err)? {
            Event::Start(e) | Event::Empty(e) => match local(e.name().as_ref()) {
                "item" => {
                    if let (Some(id), Some(href)) = (attr(&e, "id"), attr(&e, "href")) {
                        manifest.insert(id, href);
                    }
                }
                "itemref" => spine.extend(attr(&e, "idref")),
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    let mut out = String::new();
    for id in spine {
        let Some(href) = manifest.get(&id) else {
            continue;
        };
        let href = href.split('#').next().unwrap_or(href);
        let path = if base.is_empty() {
            href.to_owned()
        } else {
            format!("{base}/{href}")
        };
        let Ok(doc) = zip_entry(&mut z, &path) else {
            continue;
        };
        out.push_str(&html_to_text(&doc));
        out.push_str("\n\n");
    }
    Ok(out)
}

// ─────────────────────────── PDF ───────────────────────────

/// PDF text layer, page by page. The parser can panic on malformed files; that becomes an error.
pub fn pdf_to_text(bytes: &[u8]) -> Result<(String, u32)> {
    let owned = bytes.to_vec();
    let pages =
        std::panic::catch_unwind(move || pdf_extract::extract_text_from_mem_by_pages(&owned))
            .map_err(|_| Error::invalid("This PDF can't be read."))?
            .map_err(|e| {
                let msg = e.to_string().to_ascii_lowercase();
                if msg.contains("encrypt") || msg.contains("password") {
                    Error::invalid("This PDF is password-protected.")
                } else {
                    Error::invalid("This PDF can't be read.")
                }
            })?;
    let n = pages.len() as u32;
    let text = pages
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok((fix_visual_rtl(&tidy(&text)), n))
}

fn is_rtl(c: char) -> bool {
    matches!(c, '\u{0590}'..='\u{08FF}' | '\u{FB1D}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}')
        && !is_digit_like(c)
}

fn is_digit_like(c: char) -> bool {
    matches!(c, '\u{0660}'..='\u{0669}' | '\u{06F0}'..='\u{06F9}')
}

/// Frequent short Persian/Arabic words; a text in logical order is full of them, a text stored in
/// visual order is full of them spelled backwards.
const COMMON_RTL_WORDS: &[&str] = &[
    "از", "که", "در", "این", "را", "با", "است", "برای", "یک", "های", "تا", "شده", "بر", "آن", "هم",
    "کند", "شود", "می", "بود", "باید", "خود", "دارد", "نیز", "اين", "الى", "على", "من", "في",
];

/// Many PDFs, especially Persian ones, store right-to-left text in visual order, so extraction
/// yields each line backwards ("مايخ" for "خيام"). When the document reads backwards overall,
/// each line with RTL letters is reversed, keeping Latin and number runs in their own order.
pub fn fix_visual_rtl(text: &str) -> String {
    let norm = |w: &str| -> String {
        w.chars()
            .map(|c| match c {
                'ي' | 'ى' => 'ی',
                'ك' => 'ک',
                c => c,
            })
            .filter(|c| is_rtl(*c) && !('\u{064B}'..='\u{065F}').contains(c))
            .collect()
    };
    let (mut forward, mut backward) = (0usize, 0usize);
    for w in text.split(|c: char| !is_rtl(c)) {
        let w = norm(w);
        if w.chars().count() < 2 {
            continue;
        }
        let rev: String = w.chars().rev().collect();
        if COMMON_RTL_WORDS.iter().any(|c| norm(c) == w) {
            forward += 1;
        }
        if COMMON_RTL_WORDS.iter().any(|c| norm(c) == rev) {
            backward += 1;
        }
    }
    if backward < 5 || backward <= forward * 2 {
        return text.to_owned();
    }
    text.lines()
        .map(|line| {
            if !line.chars().any(is_rtl) {
                return line.to_owned();
            }
            // Reverse the whole line, then put each left-to-right run (Latin, digits and the
            // punctuation inside them) back in reading order.
            let rev: Vec<char> = line.chars().rev().collect();
            let mut out = String::with_capacity(line.len());
            let mut i = 0;
            while i < rev.len() {
                let ltr = |c: char| c.is_alphanumeric() && !is_rtl(c);
                if ltr(rev[i]) {
                    let mut j = i;
                    while j < rev.len()
                        && (ltr(rev[j])
                            || (matches!(rev[j], '.' | ',' | ':' | '/' | '-' | '_' | '@' | '٫')
                                && rev.get(j + 1).is_some_and(|&c| ltr(c))))
                    {
                        j += 1;
                    }
                    out.extend(rev[i..j].iter().rev());
                    i = j;
                } else {
                    out.push(match rev[i] {
                        '(' => ')',
                        ')' => '(',
                        '[' => ']',
                        ']' => '[',
                        '«' => '»',
                        '»' => '«',
                        c => c,
                    });
                    i += 1;
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_of(files: &[(&str, &str)]) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut buf);
            for (name, body) in files {
                w.start_file(*name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                w.write_all(body.as_bytes()).unwrap();
            }
            w.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn classifies_by_extension() {
        assert_eq!(classify("a.PDF"), FileClass::Document(DocKind::Pdf));
        assert_eq!(classify("voice.m4a"), FileClass::Audio);
        assert_eq!(classify("main.rs"), FileClass::Document(DocKind::Text));
        assert_eq!(classify("notes.md"), FileClass::Document(DocKind::Markdown));
        assert_eq!(audio_store_ext("x.opus"), "ogg");
    }

    #[test]
    fn plain_text_in_utf8_utf16_and_windows_1256() {
        let e = extract("a.txt", "سلام دنیا\r\nhello".as_bytes()).unwrap();
        assert_eq!(e.text, "سلام دنیا\nhello");
        assert_eq!(e.encoding, None);

        let mut u16 = vec![0xFF, 0xFE];
        for u in "salam".encode_utf16() {
            u16.extend(u.to_le_bytes());
        }
        let e = extract("a.txt", &u16).unwrap();
        assert_eq!(e.text, "salam");
        assert_eq!(e.encoding.as_deref(), Some("UTF-16LE"));

        let (bytes, _, _) = encoding_rs::WINDOWS_1256
            .encode("امروز به دکتر رفتم و گفت که حالم خوب است. فردا دوباره می‌روم.");
        let e = extract("old.txt", &bytes).unwrap();
        assert!(e.text.starts_with("امروز به دکتر"), "{}", e.text);
        assert_eq!(e.encoding.as_deref(), Some("windows-1256"));
    }

    #[test]
    fn binary_is_refused() {
        assert!(extract("x.bin", &[0, 1, 2, 3, 0, 0]).is_err());
        assert!(extract("song.mp3", b"ID3").is_err());
    }

    #[test]
    fn html_keeps_structure_and_drops_scripts() {
        let t = html_to_text(
            "<html><head><title>x</title><style>p{}</style></head><body><h2>Plan</h2>\
             <p>One &amp; two&nbsp;three</p><script>alert(1)</script><ul><li>a</li><li>b</li></ul>\
             <table><tr><td>k</td><td>v</td></tr></table></body></html>",
        );
        let t = tidy(&t);
        assert_eq!(t, "## Plan\n\nOne & two three\n\n- a\n- b\n\nk | v");
    }

    #[test]
    fn rtf_with_persian_code_page_and_unicode_escapes() {
        let rtf = br"{\rtf1\ansi\ansicpg1256{\fonttbl{\f0 Tahoma;}}{\*\generator x;}\f0 \'d3\'e1\'c7\'e3\par Hi \u1587?\u1604?\u1575?\u1605?\tab end}";
        let t = tidy(&rtf_to_text(rtf));
        assert_eq!(t, "سلام\nHi سلام\tend");
    }

    #[test]
    fn docx_headings_lists_and_tables() {
        let doc = r#"<?xml version="1.0"?><w:document xmlns:w="w"><w:body>
<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Trip</w:t></w:r></w:p>
<w:p><w:r><w:t xml:space="preserve">Packed </w:t></w:r><w:r><w:t>bags &amp; maps</w:t></w:r></w:p>
<w:p><w:pPr><w:numPr/></w:pPr><w:r><w:t>passport</w:t></w:r></w:p>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>a</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
</w:body></w:document>"#;
        let bytes = zip_of(&[("word/document.xml", doc)]);
        let e = extract("trip.docx", &bytes).unwrap();
        assert_eq!(e.kind, DocKind::Docx);
        assert_eq!(e.text, "# Trip\n\nPacked bags & maps\n\n- passport\n\na");
        // Sniffed even under a wrong extension.
        assert_eq!(extract("trip.txt", &bytes).unwrap().kind, DocKind::Docx);
    }

    #[test]
    fn odt_pptx_and_epub() {
        let odt = zip_of(&[(
            "content.xml",
            r#"<office:document-content xmlns:office="o" xmlns:text="t"><office:body><office:text>
<text:h text:outline-level="2">Title</text:h><text:p>a<text:s text:c="2"/>b</text:p>
<text:list><text:list-item><text:p>item</text:p></text:list-item></text:list>
</office:text></office:body></office:document-content>"#,
        )]);
        assert_eq!(
            extract("x.odt", &odt).unwrap().text,
            "## Title\n\na  b\n\n- item"
        );

        let pptx = zip_of(&[
            ("ppt/presentation.xml", "<p/>"),
            (
                "ppt/slides/slide2.xml",
                r#"<p:sld xmlns:a="a" xmlns:p="p"><a:p><a:r><a:t>Second</a:t></a:r></a:p></p:sld>"#,
            ),
            (
                "ppt/slides/slide1.xml",
                r#"<p:sld xmlns:a="a" xmlns:p="p"><a:p><a:r><a:t>First</a:t></a:r></a:p></p:sld>"#,
            ),
        ]);
        let e = extract("deck.pptx", &pptx).unwrap();
        assert_eq!(e.pages, Some(2));
        assert_eq!(e.text, "## Slide 1\n\nFirst\n\n## Slide 2\n\nSecond");

        let epub = zip_of(&[
            (
                "META-INF/container.xml",
                r#"<container><rootfiles><rootfile full-path="OEBPS/c.opf"/></rootfiles></container>"#,
            ),
            (
                "OEBPS/c.opf",
                r#"<package><manifest><item id="a" href="a.xhtml"/><item id="b" href="b.xhtml"/></manifest>
<spine><itemref idref="b"/><itemref idref="a"/></spine></package>"#,
            ),
            ("OEBPS/a.xhtml", "<html><body><p>Two</p></body></html>"),
            ("OEBPS/b.xhtml", "<html><body><h1>One</h1></body></html>"),
        ]);
        assert_eq!(extract("book.epub", &epub).unwrap().text, "# One\n\nTwo");
    }

    #[test]
    fn long_text_is_cut_at_a_paragraph() {
        let para = "word ".repeat(200);
        let doc = vec![para.trim(); 100].join("\n\n");
        let e = extract("long.md", doc.as_bytes()).unwrap();
        assert!(e.truncated);
        assert!(e.text.chars().count() <= MAX_IMPORT_CHARS);
        assert!(e.text.ends_with("word"));
        assert_eq!(e.total_chars, doc.chars().count());
    }

    #[test]
    fn presentation_forms_are_folded() {
        // "سلام" in isolated/final presentation forms, as some PDFs emit it.
        assert_eq!(tidy("\u{FEB3}\u{FEE0}\u{FE8E}\u{FEE1}"), "سلام");
    }

    #[test]
    fn visual_order_persian_is_put_back_in_reading_order() {
        let logical = "این متن از یک فایل است که در سال ۱۴۰۵ نوشته شد\n\
                       برای دیدن آن به www.example.com بروید (صفحه 12)\n\
                       این را هم با دقت بخوانید تا خود بدانید\nEnglish line stays";
        let visual: String = logical
            .lines()
            .map(|l| {
                if l.chars().any(is_rtl) {
                    // What a visual-order PDF yields: characters backwards, LTR runs intact.
                    fix_visual_rtl_line_for_test(l)
                } else {
                    l.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_ne!(visual, logical);
        assert_eq!(fix_visual_rtl(&visual), logical);
        // Text already in reading order is left alone.
        assert_eq!(fix_visual_rtl(logical), logical);
    }

    /// The inverse of the per-line fix (it is its own inverse).
    fn fix_visual_rtl_line_for_test(l: &str) -> String {
        let many = format!("{l}\n{}", "زا هک رد نیا ار\n".repeat(10));
        fix_visual_rtl(&many).lines().next().unwrap().to_owned()
    }

    #[test]
    fn pdf_text_layer_and_garbage() {
        let pdf = include_bytes!("../../fixtures/import/hello.pdf");
        let e = extract("hello.pdf", pdf).unwrap();
        assert_eq!(e.pages, Some(1));
        assert!(e.text.contains("Hello from a PDF"), "{}", e.text);
        assert!(extract("bad.pdf", b"%PDF-1.4 garbage").is_err());
    }
}
