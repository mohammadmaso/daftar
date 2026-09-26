# ADR-0028: File import, and the desktop tray, shortcut and compact recorder

* Status: accepted
* Date: 2026-09-26

## Context

Captures could be typed, spoken, photographed or shared as text and images. Two gaps remained:

* A document (a PDF, a Word file, notes in Markdown) or an existing recording could not be brought
  in. §4.1 already names "share-sheet import (text, URL, image, PDF)" and `RawKind::Import`
  existed, unused.
* On the desktop, the record shortcut existed only on macOS and Windows (ADR-0024). Linux had
  none, and nothing on Linux or macOS stayed reachable once the window was closed. After the
  shortcut, recording happened in the full window, on top of whatever the user was doing.

## Decisions

### Import

**The core reads; the app previews; only text enters the repo.** `daftar_core::extract` turns a
file into text: plain text in any encoding, Markdown, HTML, RTF, DOCX, ODT, PPTX, EPUB, and PDF
with a text layer. The app shows that text before anything is saved. The user can edit it, pin a
vault, and then file it. The result is an `import` capture whose body is the text, with a new
optional frontmatter field `source:` holding the file's name. `source` is written only when it is
set, so every other capture keeps the §3.1 shape byte for byte. The original file is never
committed: binaries would bloat history forever, the same reasoning as for audio in §3.1.

* **Limits.** A document over 50 MB is refused. At most 60,000 characters are filed, cut at a
  paragraph, and the preview says so. The router sees the first 8,000 characters and the file's
  name; the ingest agent sees all of the filed text.
* **Encodings.** A BOM decides the encoding first. Then valid UTF-8 is taken as is. Otherwise
  `chardetng` guesses, which matters for old Persian files in Windows-1256. Files with NUL bytes
  are refused as "not text", so binary noise is never shown.
* **Persian PDFs.** Many store right-to-left text in visual order, so extraction yields each line
  backwards. The extractor counts frequent Persian and Arabic words spelled forwards and backwards
  across the document. When the backwards spellings clearly win, each RTL line is reversed while
  Latin and number runs keep their order. Arabic presentation forms are folded to ordinary
  letters. This was checked against real Persian PDFs.
* **Scanned PDFs** have no text layer. They get a clear sentence pointing to photo capture (vision
  OCR) instead of an empty import.
* **Recordings** (MP3, M4A, MP4, WAV, OGG/Opus, FLAC, WebM, up to 25 MB, the speech-to-text
  limit) go through the voice pipeline. They are stored device-local like a voice note (`.opus` is
  stored as `.ogg`), transcribed, then filed, with `source:` set to the file's name. Other audio
  formats are refused with the list of accepted ones.

**Entry points.**
* The paperclip in the capture bar, the command palette ("Import a file") and Ctrl/Cmd+O open the
  platform file picker.
* Android: sharing a document or recording into the app opens the same preview. The file is copied
  into the app's cache and shown under its display name. Images still become photo captures.
* macOS "Open With" (`CFBundleDocumentTypes`) and Linux `daftar FILE` / `daftar --import FILE`
  (`MimeType=` in the desktop entry) do the same.
* The CLI has `daftar import <repo> <file> [--preview]`.

### Desktop: tray, shortcut, compact recorder

All of it is native runner code. There are no plugins, for the reasons in ADR-0024. It shares
the existing `daftar/hotkey` channel:

* Native → Dart: `record`, and `import` with a path (or null to pick one).
* Dart → native: `ready`, `miniRecorder`, `leaveMini`, `recording`, `labels`, `shortcut` and
  `installShortcut`.

Messages that arrive before Dart listens are queued until `ready`, for cold starts.

**Tray / menu-bar icon (Linux, macOS).** Its menu has Record a voice note, Import a file…,
Open and Quit, in the app's language (Dart sends the labels).

* Linux uses a StatusNotifierItem through libayatana-appindicator (or libappindicator). The
  library is `dlopen`ed at run time, so the app builds without it and still starts where it is
  missing, just without a tray. The .deb recommends it; the Flatpak may talk to
  `org.kde.StatusNotifierWatcher`.
* macOS uses an `NSStatusItem`. Its glyphs are drawn in code as template images, which needs no
  Xcode resources and no SF Symbols (the deployment target is 10.15).
* While a recording runs, the icon shows a red dot on Linux and a filled circle on macOS.

**Closing the window hides it** when a tray icon is showing (Linux), and always on macOS, where
the app keeps running without windows. The shortcut stays alive. Quit is in the menu. On macOS,
clicking the Dock icon brings the window back; on Linux, launching the app again does.

**Linux shortcut, Ctrl+Alt+Shift+N (the same keys as Windows).** In order of preference:
1. X11: an `XGrabKey` on the root window, including the Caps Lock and Num Lock variants.
2. Wayland: the xdg-desktop-portal `GlobalShortcuts` API (KDE Plasma, GNOME 48+). The app
   registers with the host portal registry first, so the portal knows the app id outside a
   sandbox.
3. Everywhere: the runner is now a single-instance `GApplication` (it was `NON_UNIQUE`).
   `daftar --record` reaches the running copy, so any desktop's own keyboard settings can bind it.
   The desktop entry has a "Record a voice note" action for docks and launchers.
4. GNOME without the portal (for example Ubuntu 24.04, GNOME 46) offers no API at all. There,
   Settings › Record from anywhere offers "Add it to GNOME keyboard shortcuts". It writes a custom
   keybinding through GSettings, but only when the user presses it. It isn't offered inside a
   Flatpak, which cannot write the host's settings.

Settings shows whether the shortcut works: in any app, through the desktop's settings, taken by
another app, or not available.

**Compact recorder (Linux, macOS).**
* The shortcut or the tray's Record turns the main window into a 420×150 always-on-top panel at
  the top of the screen, and recording starts at once.
* Enter, Space, the Save button or the shortcut again saves the voice note, restores the window
  and opens Today with the new note.
* Esc or Cancel discards the recording and puts the window back as it was, hidden again if it was
  hidden.
* It is one window changing shape, not a second Flutter engine. Sync and the job runner keep
  running, and it needs no multi-window support.
* On Linux, the resize waits for GTK to reprocess its client-side-decoration margins; otherwise
  both sizes come out off by the shadow width.
* On Wayland, the compositor places the panel; `keep_above` is a hint that GNOME ignores.

Windows keeps its existing behaviour: `enterMiniRecorder` returns false there, and the shortcut
records hands-free in the full window.

## Dependencies (verified 2026-09-26)

| Need | Choice | Version | Notes |
|---|---|---|---|
| PDF text (core) | `pdf-extract` | 0.12.1 | Built on `lopdf` 0.42, released 2026-09-16. Panics on malformed files are caught and reported as "can't be read". |
| DOCX/ODT/PPTX/EPUB containers (core) | `zip` | 8.6 | Deflate only (`default-features = false`). |
| XML inside them (core) | `quick-xml` | 0.42 | Streaming reader; names are `&str` in 0.42. |
| Text encodings (core) | `encoding_rs` | 0.8.42 | Already in the tree through reqwest. |
| Encoding detection (core) | `chardetng` | 1.0.0 | Mozilla's detector, for legacy code pages. |
| File picker (app) | `file_picker` | 13.1.0 | Published 2026-09-15. It uses the xdg-desktop-portal file chooser on Linux (works on Wayland and in Flatpak), UIDocumentPicker on iOS (CocoaPods, iOS 14) and SAF on Android (minSdk 21). |

Not added: `tray_manager`, `window_manager` and `hotkey_manager` (see ADR-0024 on
`hotkey_manager`). The native code is small and covers the portal and GNOME paths those plugins
don't.

## Consequences

* The Linux build links `x11` and needs `libx11-dev` at build time (it is already present
  wherever GTK is). The tray library is optional at run time.
* A second launch of the Linux app no longer opens a second window; it focuses the first.
* Not done:
  * The iOS Share Extension still accepts only text and images, so documents on iOS come in
    through the picker.
  * The macOS code is built by CI only; no Mac was available.
