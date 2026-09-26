//! File import: read a picked file into text for the preview (§4.1). Filing happens afterwards
//! through `LibraryHandle::capture_import` / `capture_audio_file`.

use std::path::Path;

use daftar_core::extract::{self, DocKind, FileClass};

pub enum ImportKind {
    Text,
    Markdown,
    Html,
    Rtf,
    Docx,
    Odt,
    Pptx,
    Epub,
    Pdf,
    Audio,
}

pub struct ImportPreview {
    pub kind: ImportKind,
    pub name: String,
    /// The text that will be filed; empty for audio.
    pub text: String,
    /// Characters in the whole document before the import limit.
    pub total_chars: u32,
    /// Whether only the first part will be filed.
    pub truncated: bool,
    /// Pages (PDF) or slides (PPTX).
    pub pages: Option<u32>,
    /// A non-UTF-8 text encoding that was detected, such as "windows-1256".
    pub encoding: Option<String>,
    pub bytes: u64,
}

/// Characters of an imported document that are filed at most.
#[flutter_rust_bridge::frb(sync)]
pub fn import_char_limit() -> u32 {
    extract::MAX_IMPORT_CHARS as u32
}

/// Reads the file at `path` for preview. Audio files are recognised (and size-checked) but not
/// read: they are transcribed after filing.
pub fn read_import(path: String) -> anyhow::Result<ImportPreview> {
    let p = Path::new(&path);
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let bytes = std::fs::metadata(p)?.len();
    if extract::classify(&name) == FileClass::Audio {
        if bytes > extract::MAX_AUDIO_BYTES {
            anyhow::bail!("This recording is too large to transcribe (over 25 MB).");
        }
        return Ok(ImportPreview {
            kind: ImportKind::Audio,
            name,
            text: String::new(),
            total_chars: 0,
            truncated: false,
            pages: None,
            encoding: None,
            bytes,
        });
    }
    let e = extract::extract_path(p).map_err(|e| match e {
        daftar_core::Error::Invalid(m) => anyhow::anyhow!(m),
        other => anyhow::anyhow!(other.to_string()),
    })?;
    Ok(ImportPreview {
        kind: match e.kind {
            DocKind::Text => ImportKind::Text,
            DocKind::Markdown => ImportKind::Markdown,
            DocKind::Html => ImportKind::Html,
            DocKind::Rtf => ImportKind::Rtf,
            DocKind::Docx => ImportKind::Docx,
            DocKind::Odt => ImportKind::Odt,
            DocKind::Pptx => ImportKind::Pptx,
            DocKind::Epub => ImportKind::Epub,
            DocKind::Pdf => ImportKind::Pdf,
        },
        name: e.name,
        text: e.text,
        total_chars: e.total_chars as u32,
        truncated: e.truncated,
        pages: e.pages,
        encoding: e.encoding,
        bytes,
    })
}
