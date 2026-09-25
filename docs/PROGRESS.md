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

## M2 — Providers + Transcribe + Ingest (2026-09-25)

**Works (core)**
* Provider adapters: `openai_compatible` (chat, tools, streaming, vision, transcription, models),
  `anthropic` (Messages, streaming, prompt caching), `gemini` (generateContent, function calls), and a
  mock provider that replays scripted responses. HTTP errors become one human sentence.
* A retry/backoff decorator around every provider, and a probe behind the Test button (latency, a
  one-token call per role, capability warnings such as "this model can't see images").
* Roles → provider + model (`AiRuntime`), with token usage and cost per op.
* Jobs: `transcribe` (STT, seals the raw file), `describe` (vision + OCR) and `ingest` (router →
  agent loop → validator → one commit + one ledger entry + log line + regenerated indexes).
* Agent loop with repair rounds (max 2), a step budget, cancellation, and context trimming that drops
  stale page reads.
* The §6.2 tools, with human-written lines protected via git blame.
* The §6.4 changeset validator, vault isolation for fiction, and a duplicate slug/alias guard.
* Op replay after sync conflicts, and a double-ingest guard re-checked under the commit lock.
* Persian-aware normalisation and FTS.
* `config.json` changes from two devices merge structurally (ADR-0016).

**Works (app)**
* Settings › AI: providers (OpenAI-compatible, Anthropic, Gemini), with the model list read from
  `/models`.
* Role → provider + model, with capability warnings and a real Test button that shows latency.
* API keys live in flutter_secure_storage only and are handed to the core per call.
* A job runner processes the queue while the app is in the foreground: after capture, after sync, on
  resume and on a slow retry timer. Filing pauses with a clear line when a role has no model.
* Today shows the filing status ("Filed to Life · Health — 4 pages updated, 1 claim to review").
  Tapping it opens the operation; a failed filing has a Retry button.

**Tests:** scenarios 1, 2, 6, 9; a Persian journal voice note that produces a journal day section,
people links and a proposed health claim; provider wire tests.

## M3 — Wiki (2026-09-25)

**Works**
* Core index: SQLite FTS5 plus a trigram index, a links table for backlinks and outlinks, a local
  graph, recent pages, folder listings and wikilink resolution. The index refreshes incrementally
  from git, and Settings › Rebuild index rebuilds it.
* Human edits are `edit:` commits without an Op-Id. They pass the secret guard and are protected from
  AI rewrites.
* CLI: `search`, `page`, `graph`, `reindex`, `edit`.
* App:
  * Wiki tab: search, recent pages and folders.
  * Page reader: our own Markdown renderer with wikilinks, callouts, inline fields, claim pills,
    per-paragraph bidi and LTR code blocks inside Persian.
  * Backlinks and a local graph.
  * An editor that warns about conflicts.
  * Desktop panes.

**Tests:** scenarios 7 and 8. Perf: 1,500 pages in debug and 5,000 in release, worst search under
250 ms in debug and 50 ms in release.

## M4 — Activity, undo, Review (2026-09-25)

**Works**
* Activity lists every op with its diff.
* Undo is a revert scoped to the op's pages and review items (ADR-0017). If later ops touched the
  same lines, a queued compensation job rewrites them.
* Other actions on a filing:
  * Move to another vault.
  * Re-run with a note.
  * Include an excluded capture.
* Review resolutions (confirm, reject, edit a claim) are ops of their own (ADR-0018), and review
  cards for claims that no longer exist are pruned.
* CLI: `activity`, `diff`, `undo`, `move`, `rerun`, `include`, `review`.
* App: the Activity screen, the op detail with diffs, and the Review stack (swipe or buttons). A
  Review count shows on Today.

**Tests:** scenarios 4 and 5 (undo the middle op → compensate; undo and redo; move; confirm and
reject).

## M5 — Ask (2026-09-25)

**Works**
* A read-only agent that streams answers. Citations are resolved in code against pages it actually
  read, so it cannot cite a page that doesn't exist (ADR-0019).
* Scopes: everything, one vault, or one story (fiction stays isolated). Image input is supported.
* Save an answer to the wiki (it goes through ingest). Story mode can save a draft into the story.
* Crisis phrases in a question show the Talk to someone card with curated helplines (§4.7).
* CLI: `ask`.
* App: the Ask screen with streaming answers, citation chips, a scope picker, attaching a photo and
  save actions.

**Tests:** answers cite real pages and do not invent; story scope isolation; saved answers are
filed.

## M6 — Voice (2026-09-25)

**Works**
* Energy VAD, a sentence chunker for TTS, and barge-in that stops speech within 200 ms.
* Streaming STT → LLM → TTS, with per-language voices.
* "Remember that…" files a capture, and the conversation transcript is saved as a capture.
* App: voice mode with 16 kHz PCM from `record` and playback through `audioplayers`. It shows live
  captions and has mute and end buttons. The screen stays awake, and reduced motion stops the
  breathing animation.

**Tests:** scenario 11 (barge-in timing; remember-that files a capture); widget tests for the mic,
playback, barge-in and ending.

**Not yet:** Android foreground service for voice with the screen off. Voice sessions don't use
MCP tools, so there are no spoken approvals.

## M7 — MCP (2026-09-25)

**Works**
* Transports, on rmcp 3.4.1 (ADR-0021):
  * Streamable HTTP.
  * Legacy SSE.
  * stdio, on desktop only.
* Auth:
  * None.
  * Static headers.
  * OAuth with PKCE, dynamic client registration and refresh. Desktop uses a loopback redirect;
    phones use `daftar://oauth/callback` through the system browser.
* Per-device secrets live in secure storage; server config syncs.
* Tool policies (ask every time, auto for read-only, always allow), with in-chat approval cards.
* Tools are exposed to Ask as `mcp__server__tool`.
* CLI: `mcp`.
* App: Settings › Outside tools, with add/edit, status, the tool list, sign in and remove.

**Tests:** scenario 10 (OAuth PKCE + DCR + refresh + policies; static token over legacy SSE).

## M8 — Reflect, lint, wellbeing (2026-09-25)

**Works**
* Daily reflection and weekly review, written by the reflect role, cited and filed to Journal
  (ADR-0022).
* A pattern claim needs at least three captures as evidence.
* Notifications are neutral. A heavy day raises the Talk to someone card and never puts details into
  a notification.
* Lint combines deterministic checks (broken links, orphans, frontmatter, duplicate slugs and
  aliases, oversized pages, stuck captures) with model findings (contradictions, stale claims,
  missing pages, merges, splits) whose quotes are verified. Each finding becomes a Review card once. Lint runs after every
  few ingests or weekly.
* The §12 secret guard covers sync, the validator, human edits and the changeset. Captured secrets
  are held back until redacted.
* CLI: `reflect`, `lint`, and `eval`, which runs the JSON fixtures in `core/fixtures/eval` (§15).
* App:
  * Settings › Reflect: daily and time, weekly and day, notifications with a permission request, and
    the helpline country.
  * Check the wiki now.
  * Reflections and lint are queued on open, on resume and every ten minutes. Local notifications
    use flutter_local_notifications.
  * The help card shows on Today after a heavy day.

**Tests:** a daily summary once, with a neutral notification; a heavy day gets care, not details;
weekly review citations and the pattern threshold; lint findings surface once; the eval harness;
Reflect settings, notification and help-card widget tests.

**Not yet:** reflections that fall due while the app is closed run the next time it opens. Background
execution comes with M9.

## Status after M8

* Rust: 118 tests (unit + scenario + eval + perf), clippy clean with `-D warnings`.
* Flutter: 147 tests, 64 goldens rendered on Linux (`tools/update_goldens.sh` runs them in Docker
  from any host).
* **Not verified on devices since M1:** no Xcode or Android SDK on the development machine used for
  M2–M8. Everything above is covered by the core scenario tests and widget tests with fakes, not by
  a run on a phone.
* Core error messages are English in both languages.
