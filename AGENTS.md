# AGENTS.md

Guide for coding agents working in this repository. Product spec: the Daftar build brief
(referenced as §n). Plan: `docs/PLAN.md`. Status: `docs/PROGRESS.md`. Decisions: `docs/adr/`.

## Layout

```
app/                 Flutter app (package `daftar`)
  lib/app/           app root, router, appearance state, feature gates, identity (product name)
  lib/design/        tokens.dart, theme.dart, icons.dart, components/ — the only place for visual values
  lib/features/<f>/  one folder per feature: capture, wiki, ask, voice, reflect, activity, settings
  lib/l10n/          app_en.arb (template), app_fa.arb → generated `L10n`
  lib/src/rust/      GENERATED bindings (flutter_rust_bridge) — never edit by hand
  rust_builder/      cargokit plugin that builds core/daftar_bridge for each platform
  test/              widget, design and golden tests (goldens under test/golden/goldens)
core/                Rust workspace
  daftar_core/       all logic; no FFI types
  daftar_bridge/     FRB API surface (src/api/*.rs) — thin wrappers + DTOs
  daftar_cli/        `daftar` binary: same ops against a repo path
  fixtures/          sample repos, recorded provider responses
  prompts/           versioned system prompts
docs/                PLAN, PROGRESS, adr/, design/
```

## Commands

```sh
# Rust
cd core && cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cargo run -p daftar_cli -- info

# Bindings (after changing core/daftar_bridge/src/api/**)
cd app && flutter_rust_bridge_codegen generate        # codegen pinned to 2.13.0

# Flutter
cd app && flutter analyze && flutter test
flutter test --update-goldens test/golden             # only on Linux; goldens are Linux-rendered
flutter run -d linux --dart-define=DAFTAR_PREVIEW=true  # preview surfaces (design gallery)
flutter build linux | apk | windows | macos | ios --no-codesign
```

On machines with an HTTP proxy set, `flutter test` needs
`NO_PROXY=127.0.0.1,localhost,::1 no_proxy=127.0.0.1,localhost,::1`, otherwise the test runner's
local websocket is sent to the proxy and fails with HTTP 403.

## Conventions

* **Repo is truth, SQLite is cache.** Never store user knowledge only in SQLite.
* **All AI writes go through the changeset pipeline** (§6.4): one op → one commit → one ledger file.
* **Atomic file writes** via `daftar_core::fsutil::atomic_write`.
* **Repo paths** come from `daftar_core::layout`; always `/`-separated, repo-relative.
* **No fake features.** Unfinished UI is compiled out via `lib/app/features.dart` (ADR-0004).
* **Design:** only tokens from `lib/design/tokens.dart`; custom components from `lib/design/`; no
  Material widgets for visible chrome, no Material icons, no emoji, no gradients, no "AI" sparkle.
  Use `EdgeInsetsDirectional`/`AlignmentDirectional` so layouts mirror in Persian.
* **Strings:** every user-visible string goes through ARB in both `en` and `fa`. Copy is short and
  specific ("Filed to Life and Health").
* **Product name** lives only in `app/lib/app/identity.dart` and `daftar_core::APP_NAME`.
* **Secrets** never enter the repo, logs, or SQLite; only platform secure storage.
* **Dependencies:** verify maintenance and latest version before adding; record in an ADR.
* Record non-obvious decisions as ADRs (`docs/adr/NNNN-title.md`), update `docs/PROGRESS.md` after
  each milestone.
