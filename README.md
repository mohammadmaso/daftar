# Daftar

**A notebook that files itself.** Daftar (دفتر, "notebook") is a local-first, bilingual
(English / Persian), voice-first personal wiki. You talk, type or take a photo. Each capture is
saved at once to a private Git repository that you own. The AI models you choose then file it into
an interlinked Markdown wiki: your journal, the people in your life, your health, your work and
your stories. You can read, edit and correct everything, and every change the assistant makes can
be undone.

It follows Andrej Karpathy's **LLM Wiki** pattern. Instead of running RAG over raw notes, the model
keeps a persistent wiki up to date. When a new source arrives, it updates the pages it affects,
adds links, flags contradictions and revises summaries. Knowledge is compiled once and kept
current, not worked out again for every question.

> "Daftar" is a codename. The product name lives only in `app/lib/app/identity.dart` and
> `daftar_core::APP_NAME`.

---

## Highlights

* **Capture with near-zero friction.** Hold to record voice (Persian, English or both), type a
  note, or snap a photo. Captures are saved at once and work offline.
* **Your repo is the truth.** Everything is plain Markdown in a Git repository you control, synced
  over HTTPS (token) or SSH. SQLite is only a rebuildable cache.
* **Bring your own models.** OpenAI-compatible, Anthropic and Gemini providers, with one model per
  role (routing, filing, chat, voice, vision, speech to text, text to speech, reflection, upkeep).
  API keys stay in the device's secure storage and never reach the repo.
* **Auditable AI.** Every AI write goes through a validated changeset: one operation, one commit,
  one ledger file. Activity shows each diff, and undo is a scoped revert. Lines you wrote yourself
  are protected from AI rewrites.
* **Ask.** Streaming answers with citations that are checked against the pages the model actually
  read. Answers can be scoped to a vault or a story, and saved back into the wiki.
* **Talk.** Real-time voice conversation (STT → LLM → TTS) with barge-in. "Remember that…" files a
  capture.
* **Reflect and upkeep.** Daily reflections and weekly reviews. Lint finds broken links, orphans,
  contradictions and stale claims, and turns them into Review cards.
* **Outside tools.** MCP client (Streamable HTTP, SSE, stdio) with OAuth PKCE and per-tool approval
  policies.
* **Bilingual all the way down.** Full RTL mirroring, Persian-aware search, Jalali dates and
  per-paragraph text direction.
* **Everywhere.** Linux, Android, Windows, macOS and iOS from one Flutter codebase and one Rust core.

## How it works

```
┌──────────────────────────── Flutter app (app/) ────────────────────────────┐
│ capture · wiki · ask · voice · reflect · activity · settings               │
│ Riverpod · go_router · gen-l10n (en, fa) · custom design system            │
└───────────────▲────────────────────────────────────────────────────────────┘
                │ flutter_rust_bridge 2 (core/daftar_bridge)
┌───────────────┴────────────── Rust workspace (core/) ──────────────────────┐
│ daftar_core: repo · capture · jobs · sync · index · wiki · changeset ·     │
│              ledger · providers · agent · ops · mcp · voice · vaults       │
│ daftar_cli:  the same operations against a repo path                       │
│ prompts/:    versioned system prompts, embedded at compile time            │
└────────────────────────────────────────────────────────────────────────────┘
        │ git (HTTPS / SSH)        │ HTTPS (model providers)        │ MCP servers
```

The wiki has three layers. **Raw sources** are immutable. **The wiki** is Markdown that the model
maintains. **The schema** (`SCHEMA.md`) is a document you can edit that tells the model how the
wiki works. A library repository looks like this:

```
SCHEMA.md                 how the wiki is organised (user-editable)
raw/YYYY/MM/DD/…md        immutable captures; raw/assets/ holds images
vaults/<vault>/…md        wiki pages, one folder per vault (Life, Stories, …)
log/YYYY-MM.md            chronological log of operations
.daftar/config.json       shared settings: providers, roles, vaults, MCP servers (no secrets)
.daftar/ledger/…json      one record per AI operation
.daftar/devices/…json     registered devices
```

The full design is in [`docs/PLAN.md`](docs/PLAN.md), and the reasons behind it are in the
[ADRs](docs/adr/).

## Repository layout

```
app/                 Flutter app (package `daftar`)
  lib/app/           app root, router, appearance, feature gates, product identity
  lib/design/        design tokens, theme, icons and components
  lib/features/      capture, wiki, ask, voice, reflect, activity, settings
  lib/l10n/          app_en.arb, app_fa.arb
  lib/src/rust/      generated bindings (never edit by hand)
  rust_builder/      cargokit plugin that builds the Rust bridge for each platform
  test/              widget, design and golden tests
core/                Rust workspace
  daftar_core/       all logic, no FFI types
  daftar_bridge/     flutter_rust_bridge API surface: thin wrappers and DTOs
  daftar_cli/        `daftar` command-line tool
  fixtures/          sample repos, recorded provider responses, eval cases
  prompts/           versioned system prompts
docs/                plan, progress, ADRs, design style guide, user guides, packaging
packaging/           Linux (AppImage, .deb, Flatpak), macOS (DMG), Windows (MSIX), icons
tools/               local git HTTP server, golden updater, iOS extension setup
.github/workflows/   CI (ci.yml) and tag-triggered releases (release.yml)
```

## Getting started

### Prerequisites

* **Flutter 3.41.1** (pinned, see [ADR-0003](docs/adr/0003-flutter-sdk-pin.md))
* **Rust** (stable) with `cargo`
* **flutter_rust_bridge_codegen 2.13.0**, only needed when you change the bridge API
* The usual platform toolchains for your targets (Linux desktop libraries, Android SDK, Xcode,
  Visual Studio)

If Flutter is not on your `PATH`, prefix commands with
`PATH=$HOME/flutter/bin:$HOME/.cargo/bin:$PATH`.

### Run the app

```sh
cd app
flutter pub get
flutter run -d linux            # or: -d android, -d macos, -d windows, …
```

The Rust core is built automatically through cargokit. To browse the design gallery and other
preview surfaces:

```sh
flutter run -d linux --dart-define=DAFTAR_PREVIEW=true
```

On first launch, connect a private Git repository (HTTPS + token, or an SSH key the app generates)
or start on the device only and connect later. Then add a provider and pick models in Settings. See
the user guide: [English](docs/user-guide.en.md) · [فارسی](docs/user-guide.fa.md).

### Use the CLI

`daftar_cli` exposes the same operations as the app against a repository path. It is handy for
scripting, debugging and evals.

```sh
cd core
cargo run -p daftar_cli -- info
cargo run -p daftar_cli -- init ~/notes --device laptop
cargo run -p daftar_cli -- capture ~/notes "Met Sara for coffee; she starts at the clinic in May."

# Add a provider (prints its id), point a role at it, then process the queue
cargo run -p daftar_cli -- provider ~/notes add --name OpenAI --kind openai_compatible
cargo run -p daftar_cli -- role ~/notes ingest <provider-id> <model>
DAFTAR_API_KEY_<PROVIDER_ID>=sk-… cargo run -p daftar_cli -- jobs ~/notes

cargo run -p daftar_cli -- search ~/notes "Sara"
DAFTAR_API_KEY_<PROVIDER_ID>=sk-… cargo run -p daftar_cli -- ask ~/notes "What is Sara up to?"
cargo run -p daftar_cli -- --help
```

Other commands: `clone`, `remote`, `photo`, `today`, `status`, `sync`, `test`, `page`, `graph`,
`reindex`, `edit`, `activity`, `diff`, `undo`, `move`, `rerun`, `include`, `review`, `mcp`,
`vault`, `reflect`, `lint`, `eval` and `keygen`. Credentials come from the environment only:
`DAFTAR_API_KEY_<ID>`, `DAFTAR_GIT_USER` / `DAFTAR_GIT_TOKEN`, `DAFTAR_SSH_KEY_FILE` /
`DAFTAR_SSH_PASSPHRASE` and `DAFTAR_MCP_SECRETS_<ID>`.

## Development

```sh
# Rust: format, lint, test
cd core && cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace

# Regenerate bindings after changing core/daftar_bridge/src/api/**
cd app && flutter_rust_bridge_codegen generate

# Flutter: analyze and test
cd app && flutter analyze && flutter test

# Goldens are rendered on Linux
cd app && flutter test --update-goldens test/golden   # on Linux
tools/update_goldens.sh [--plain-name X]              # in Docker, from macOS/Windows
```

Behind an HTTP proxy, `flutter test` needs
`NO_PROXY=127.0.0.1,localhost,::1 no_proxy=127.0.0.1,localhost,::1`.

To test sync against a local remote (including from the Android emulator via `adb reverse`), use
`tools/git_http_server.py`.

### Conventions

* **The repo is truth, SQLite is cache.** User knowledge is never stored only in SQLite.
* **All AI writes go through the changeset pipeline.** One op, one commit, one ledger file.
* **No fake features.** Unfinished UI is compiled out through `app/lib/app/features.dart`
  ([ADR-0004](docs/adr/0004-feature-gating.md)).
* **Design** uses only tokens from `app/lib/design/tokens.dart` and components from
  `app/lib/design/`: no Material chrome or icons, no emoji, no gradients. Layouts use directional
  insets so they mirror in Persian. See [the style guide](docs/design/style-guide.md).
* **Strings** go through ARB in both `en` and `fa`.
* **Secrets** never enter the repo, logs or SQLite, only platform secure storage.
* **Dependencies** are checked for maintenance before adoption and recorded in an ADR.
* **Decisions** that aren't obvious get an ADR in `docs/adr/`.

[`AGENTS.md`](AGENTS.md) has the same guide for coding agents.

## Releases

Pushing a tag such as `v0.1.0` runs `.github/workflows/release.yml`. It builds every package and
attaches it to a draft GitHub release:

| Platform | Artifacts |
|---|---|
| Linux | AppImage, `.deb`, Flatpak |
| Android | APK, AAB |
| iOS | IPA (unsigned) |
| macOS | DMG (signed and notarized when the secrets are set) |
| Windows | MSIX |

Signing is driven by repository secrets. Without them, packages are unsigned or debug-signed and
only fit for testing. See [`docs/packaging.md`](docs/packaging.md) for local builds and the list of
secrets.

## Status

Milestones M0–M9 are complete: capture and sync, providers and ingest, the wiki, activity and undo,
Ask, voice, MCP, reflection and lint, then polish and packaging. The graph view and user-managed
vaults came after. The core has about 120 Rust tests, including the scenario and eval suites, and
the app has over 200 Flutter tests and 68 goldens.

Since M1, the app has been checked through core scenario tests and widget tests with fakes, not on
physical phones. iOS, macOS and Windows are built in CI. Details and known issues are in
[`docs/PROGRESS.md`](docs/PROGRESS.md).

## Documentation

| Document | What it covers |
|---|---|
| [`docs/BRIEF.md`](docs/BRIEF.md) | The product specification |
| [`docs/PLAN.md`](docs/PLAN.md) | Architecture, milestones, risks |
| [`docs/PROGRESS.md`](docs/PROGRESS.md) | What works, what doesn't yet, known issues |
| [`docs/adr/`](docs/adr/) | Architecture decision records |
| [`docs/design/style-guide.md`](docs/design/style-guide.md) | Visual language |
| [`docs/user-guide.en.md`](docs/user-guide.en.md) · [`fa`](docs/user-guide.fa.md) | User guides |
| [`docs/packaging.md`](docs/packaging.md) | Packaging and signing |
