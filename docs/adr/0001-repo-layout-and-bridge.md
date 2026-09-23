# ADR-0001: Monorepo layout and Rust bridge placement

* Status: accepted
* Date: 2026-09-23

## Context
The brief fixes `/app` (Flutter) and `/core` (Rust workspace with `daftar_core` and `daftar_cli`).
flutter_rust_bridge's `integrate` command generates a crate inside the Flutter app (`app/rust`) and a
cargokit-based `rust_builder` plugin that compiles it for each platform.

## Decision
* `core/` is one Cargo workspace: `daftar_core` (pure logic, no FFI), `daftar_cli` (binary `daftar`),
  `daftar_bridge` (the FRB crate: thin API functions and DTOs only).
* `app/rust_builder` (cargokit) points at `../core/daftar_bridge` on every platform
  (Android gradle, Linux/Windows CMake, iOS/macOS podspecs).
* Generated Dart bindings live in `app/lib/src/rust/` and are committed; CI fails if regenerating
  changes them.
* Bridge DTOs are separate from core types so the core never depends on FRB annotations.

## Consequences
Core is testable with plain `cargo test` and the CLI. Adding an API = core function + bridge wrapper +
`flutter_rust_bridge_codegen generate`.
