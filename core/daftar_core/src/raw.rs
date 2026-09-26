//! Raw captures (§3.1): one immutable Markdown file per capture with a globally unique name.
//!
//! A capture is *sealed* once its body is final. Text captures are sealed on creation; voice and
//! photo captures are sealed when the transcribe / describe job fills the body on the capturing
//! device (the only device holding the audio / original). Only sealed captures are committed, so a
//! raw file never changes content after it has been shared. `status` is the only field that may
//! change afterwards.

use std::fs;

use jiff::Zoned;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::frontmatter::{self, quote};
use crate::fsutil::atomic_write;
use crate::layout::{self, Date};
use crate::library::{Library, LocalDevice};
use crate::{Error, Result, lang, time};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RawKind {
    Voice,
    Text,
    Photo,
    VoiceConversation,
    ChatAnswer,
    Import,
}

impl RawKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Voice => "voice",
            Self::Text => "text",
            Self::Photo => "photo",
            Self::VoiceConversation => "voice-conversation",
            Self::ChatAnswer => "chat-answer",
            Self::Import => "import",
        }
    }

    /// Whether the body is produced later by a model (transcription / vision).
    pub fn needs_model_body(self) -> bool {
        matches!(self, Self::Voice | Self::Photo)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RawStatus {
    Pending,
    Ingested,
    Excluded,
}

impl RawStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ingested => "ingested",
            Self::Excluded => "excluded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawMeta {
    pub id: String,
    pub kind: RawKind,
    pub captured_at: String,
    pub device: String,
    #[serde(default)]
    pub lang: Vec<String>,
    #[serde(default)]
    pub vault_hint: Option<String>,
    #[serde(default)]
    pub assets: Vec<String>,
    #[serde(default)]
    pub transcript_model: Option<String>,
    /// The imported file's name, for `import` captures and imported recordings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub status: RawStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawItem {
    /// Repo-relative path.
    pub path: String,
    pub meta: RawMeta,
    pub body: String,
}

impl RawItem {
    pub fn id(&self) -> Ulid {
        self.meta.id.parse().expect("raw ids are valid ULIDs")
    }
}

#[derive(Debug, Clone, Default)]
pub struct NewCapture {
    pub text: String,
    pub vault_hint: Option<String>,
    pub assets: Vec<String>,
    pub source: Option<String>,
}

fn render(meta: &RawMeta, body: &str) -> String {
    let list = |v: &[String]| {
        if v.is_empty() {
            "[]".to_owned()
        } else {
            format!(
                "[{}]",
                v.iter().map(|s| quote(s)).collect::<Vec<_>>().join(", ")
            )
        }
    };
    let opt = |v: &Option<String>| v.as_deref().map(quote).unwrap_or_else(|| "null".into());
    // `source` is written only when set, so other captures keep the §3.1 shape exactly.
    let source = meta
        .source
        .as_deref()
        .map(|s| format!("source: {}\n", quote(s)))
        .unwrap_or_default();
    let mut out = format!(
        "---\nid: {}\nkind: {}\ncaptured_at: {}\ndevice: {}\nlang: {}\nvault_hint: {}\nassets: {}\ntranscript_model: {}\n{source}status: {}\n---\n",
        meta.id,
        meta.kind.as_str(),
        meta.captured_at,
        quote(&meta.device),
        list(&meta.lang),
        opt(&meta.vault_hint),
        list(&meta.assets),
        opt(&meta.transcript_model),
        meta.status.as_str(),
    );
    let body = body.trim_end();
    if !body.is_empty() {
        out.push('\n');
        out.push_str(body);
        out.push('\n');
    }
    out
}

pub fn parse(path: &str, doc: &str) -> Result<RawItem> {
    let (yaml, body) = frontmatter::split(doc);
    let yaml = yaml.ok_or_else(|| Error::invalid(format!("{path}: missing frontmatter")))?;
    let meta: RawMeta =
        serde_saphyr::from_str(yaml).map_err(|e| Error::invalid(format!("{path}: {e}")))?;
    meta.id
        .parse::<Ulid>()
        .map_err(|_| Error::invalid(format!("{path}: invalid id")))?;
    Ok(RawItem {
        path: path.to_owned(),
        meta,
        body: body.trim_start_matches(['\r', '\n']).trim_end().to_owned(),
    })
}

/// Writes a new capture. This is the latency-critical path: one small file write, no git.
pub fn create(
    lib: &Library,
    device: &LocalDevice,
    now: &Zoned,
    kind: RawKind,
    capture: NewCapture,
) -> Result<RawItem> {
    let id = Ulid::from_datetime(std::time::SystemTime::from(now.timestamp()));
    create_with_id(lib, device, now, kind, capture, id)
}

pub(crate) fn create_with_id(
    lib: &Library,
    device: &LocalDevice,
    now: &Zoned,
    kind: RawKind,
    capture: NewCapture,
    id: Ulid,
) -> Result<RawItem> {
    if !kind.needs_model_body() && capture.text.trim().is_empty() {
        return Err(Error::invalid("empty capture"));
    }
    let rel = layout::raw_capture(time::date_of(now), &time::compact(now), &device.id, id);
    let meta = RawMeta {
        id: id.to_string(),
        kind,
        captured_at: time::rfc3339(now),
        device: device.id.clone(),
        lang: lang::detect(&capture.text),
        vault_hint: capture.vault_hint,
        assets: capture.assets,
        transcript_model: None,
        source: capture.source,
        status: RawStatus::Pending,
    };
    let doc = render(&meta, &capture.text);
    atomic_write(&lib.path(&rel), doc.as_bytes())?;
    parse(&rel, &doc)
}

pub fn read(lib: &Library, rel: &str) -> Result<RawItem> {
    let doc = fs::read_to_string(lib.path(rel))?;
    parse(rel, &doc)
}

/// Seals a voice/photo capture by filling its body. Allowed exactly once, before it is committed.
pub fn seal(lib: &Library, rel: &str, body: &str, model: Option<&str>) -> Result<RawItem> {
    let mut item = read(lib, rel)?;
    if !item.body.is_empty() {
        return Err(Error::invalid(format!("{rel} is already sealed")));
    }
    item.meta.lang = lang::detect(body);
    item.meta.transcript_model = model.map(str::to_owned);
    let doc = render(&item.meta, body);
    atomic_write(&lib.path(rel), doc.as_bytes())?;
    parse(rel, &doc)
}

pub fn set_status(lib: &Library, rel: &str, status: RawStatus) -> Result<()> {
    let doc = fs::read_to_string(lib.path(rel))?;
    let updated = frontmatter::replace_scalar(&doc, "status", status.as_str())
        .ok_or_else(|| Error::invalid(format!("{rel}: no status field")))?;
    if updated != doc {
        atomic_write(&lib.path(rel), updated.as_bytes())?;
    }
    Ok(())
}

/// All captures filed under `date`, oldest first.
pub fn list_day(lib: &Library, date: Date) -> Result<Vec<RawItem>> {
    let dir = format!(
        "{}/{:04}/{:02}/{:02}",
        layout::RAW_DIR,
        date.year,
        date.month,
        date.day
    );
    let mut items = Vec::new();
    let Ok(entries) = fs::read_dir(lib.path(&dir)) else {
        return Ok(items);
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".md") || name.starts_with('.') {
            continue;
        }
        let rel = format!("{dir}/{name}");
        match read(lib, &rel) {
            Ok(item) => items.push(item),
            Err(err) => tracing::warn!("skipping unreadable raw file {rel}: {err}"),
        }
    }
    items.sort_by(|a, b| a.meta.id.cmp(&b.meta.id));
    Ok(items)
}

/// Locates a capture by id. The ULID's timestamp gives the day (±1 for time zones).
pub fn find(lib: &Library, id: Ulid) -> Result<Option<RawItem>> {
    let ts = jiff::Timestamp::from_millisecond(id.timestamp_ms() as i64)?;
    let suffix = format!("-{id}.md");
    for delta in [0i64, -1, 1] {
        let day = ts
            .checked_add(jiff::SignedDuration::from_hours(24 * delta))?
            .to_zoned(jiff::tz::TimeZone::UTC);
        let date = time::date_of(&day);
        let dir = format!(
            "{}/{:04}/{:02}/{:02}",
            layout::RAW_DIR,
            date.year,
            date.month,
            date.day
        );
        let Ok(entries) = fs::read_dir(lib.path(&dir)) else {
            continue;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(&suffix) {
                return read(lib, &format!("{dir}/{name}")).map(Some);
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{device, lib_in, zoned};

    #[test]
    fn create_read_status_round_trip() {
        let (_d, lib) = lib_in();
        let now = zoned("2026-09-23T14:15:02+03:30[Asia/Tehran]");
        let item = create(
            &lib,
            &device("pixel-8"),
            &now,
            RawKind::Text,
            NewCapture {
                text: "جلسه با Sara خوب بود".into(),
                vault_hint: Some("life".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            item.path
                .starts_with("raw/2026/09/23/20260923T141502-pixel-8-")
        );
        assert_eq!(item.meta.lang, vec!["fa", "en"]);
        assert_eq!(item.meta.captured_at, "2026-09-23T14:15:02+03:30");

        set_status(&lib, &item.path, RawStatus::Ingested).unwrap();
        let again = read(&lib, &item.path).unwrap();
        assert_eq!(again.meta.status, RawStatus::Ingested);
        assert_eq!(again.body, item.body);
        assert_eq!(find(&lib, item.id()).unwrap().unwrap().path, item.path);
        assert_eq!(list_day(&lib, time::date_of(&now)).unwrap().len(), 1);
    }

    #[test]
    fn voice_is_sealed_once() {
        let (_d, lib) = lib_in();
        let now = zoned("2026-09-23T08:00:00+02:00[Europe/Berlin]");
        let item = create(
            &lib,
            &device("laptop"),
            &now,
            RawKind::Voice,
            NewCapture::default(),
        )
        .unwrap();
        assert!(item.body.is_empty());
        let sealed = seal(&lib, &item.path, "Slept badly.", Some("p1/whisper")).unwrap();
        assert_eq!(sealed.meta.lang, vec!["en"]);
        assert_eq!(sealed.meta.transcript_model.as_deref(), Some("p1/whisper"));
        assert!(seal(&lib, &item.path, "again", None).is_err());
    }

    #[test]
    fn empty_text_is_rejected() {
        let (_d, lib) = lib_in();
        let now = zoned("2026-09-23T08:00:00+00:00[UTC]");
        assert!(
            create(
                &lib,
                &device("d"),
                &now,
                RawKind::Text,
                NewCapture::default()
            )
            .is_err()
        );
    }
}
