# Progress

## M0 — Foundations (2026-09-23)

**Works**
* Monorepo: `app/` (Flutter 3.41.1) + `core/` Rust workspace (`daftar_core`, `daftar_bridge`, `daftar_cli`).
* flutter_rust_bridge 2.13 wired through cargokit to `core/daftar_bridge`; Settings › About shows the
  version reported by the Rust core.
* Design system: tokens (spacing, radii, motion, palette light/dark, per-script type scale), custom
  stroke icons, components (Pressable, DButton, DIconButton, DChip, DSegmented, DSwitch, DStatusPill,
  DSurface, DSection, DListRow, DTextField, DHairline, DPage). Bundled Inter/Vazirmatn/JetBrains Mono.
* i18n en/fa via gen-l10n; full RTL mirroring; live language/theme/text-size switching, persisted per device.
* Core foundations: `atomic_write`, repo path layout, ids/slugs; `daftar info`.
* Tests: 5 Rust unit tests; 47 Flutter tests (appearance behaviour, 200 % text scale in en/fa, WCAG AA
  contrast for every text/surface pair); 12 goldens (settings phone/desktop + gallery × light/dark × en/fa).
* CI: Rust fmt/clippy/test; bindings-up-to-date check; analyze; tests; builds for Linux, Android,
  Windows, macOS, iOS (unsigned).

**Verified locally:** Linux debug build launches; Android build — see below.
**Not verified locally:** Windows, macOS, iOS (CI only).

**Shipped UI surface at M0:** Settings only (ADR-0004). Design gallery with `DAFTAR_PREVIEW=true`.

## M1 — Capture + Repo + Sync (2026-09-23)

**Works**
* Core: library init/clone (empty remotes are initialised and pushed), device registration, raw
  captures (text / photo / voice) with immutable files and sealing (ADR-0009), image normalisation
  (EXIF orientation, ≤1600 px, JPEG q80, metadata stripped), device-local audio with 14-day pruning,
  durable SQLite job queue (FIFO, backoff, network wait, crash recovery), ledger + review-item file
  formats, full sync algorithm §5.4 incl. double-ingest guard, op-replay detection, human-conflict
  callouts + Review items, union-merged logs, safe checkout, push retry ×3 (ADR-0008),
  in-app ed25519 key generation, HTTPS token and SSH-key auth.
* CLI: `init`, `clone`, `remote`, `capture`, `photo`, `today`, `status`, `sync`, `keygen`
  (credentials from env only).
* App: onboarding (connect via HTTPS token or generated SSH key, or start local-only), device naming,
  Today timeline (per-paragraph direction, Jalali dates in Persian), capture bar (tap → text sheet,
  hold → record, slide to cancel, slide up to lock, camera/gallery, per-capture vault pin), sync
  indicator (synced / n changes / syncing / offline / needs attention / this device only),
  opportunistic sync (foreground, after capture debounced 10 s, every 5 min, pull to refresh),
  Settings › Repository (remote, device, branch, folder, sync now, connect later), desktop nav rail.
  Errors are shown as one sentence, never a backtrace.
* Tests: Rust 24 unit + 9 scenario tests — scenario 1 without AI, scenario 2 raw part, scenario 3
  (both orderings), op replay, human conflict → callout, external edits, unsealed voice, capture
  latency (< 100 ms worst of 50). Flutter 84 tests incl. onboarding/capture flows and 24 goldens.
  Integration test (real Rust core, Linux): onboarding → capture → sync → second device sees it.

**Verified on Android emulator (API 35, x86_64):** onboarding against a loopback smart-HTTP remote
(`tools/git_http_server.py` + `adb reverse`), capture while offline ("Offline" shown calmly), pull to
refresh pushes, a fresh desktop clone lists the note; microphone permission + hold-to-record stores
audio locally and keeps the capture unsealed.

**Not yet**
* Transcription/description/filing — M2 (captures show "Saved" and honest pending text).
* Background sync via workmanager/BGTaskScheduler — deferred to M9 (foreground triggers only).
* Audio commit option (Opus 16 kbps) — M6 with the voice pipeline.
* Not verified locally: iOS, macOS, Windows (CI builds only). SSH against a real host not exercised
  (unit-tested key generation; libssh2 path covered in CI builds only).

**Known issues**
* The "n changes to sync" counter can briefly include voice captures that are waiting for
  transcription; it corrects itself after the next sync attempt.
* Android logs SELinux `link` denials from libgit2; harmless (libgit2 falls back to rename).
