# ADR-0002: Dependency choices (verified 2026-09-23)

* Status: accepted
* Date: 2026-09-23

Versions checked against crates.io / pub.dev on the date above.

| Need | Choice | Version | Notes |
|---|---|---|---|
| FFI | flutter_rust_bridge | 2.13.0 (latest stable; 2.14 beta skipped) | codegen pinned identically |
| Git | git2 | 0.21 | `vendored-libgit2`, `vendored-openssl`, `ssh`. `gix` 0.87 still lacks a complete push/merge story for our needs → not adopted. |
| SQLite | rusqlite | 0.40 | `bundled` (includes FTS5) |
| MCP | rmcp | 3.4 | official SDK; client + transports |
| HTTP | reqwest | 0.13 | rustls |
| Async | tokio | 1.53 | |
| OAuth | oauth2 | 5.0 | discovery/DCR implemented by us |
| IDs | ulid | 3.0 | note: `Ulid::generate()` replaced `new()` in v3 |
| VAD | ort 2.0 is still RC → decision deferred to M6 (ADR then) | | energy VAD fallback is always present |
| State | flutter_riverpod | 3.3.x | 3.4 needs Dart 3.12 (see ADR-0003) |
| Routing | go_router | 17.5 | 18.x needs Flutter 3.44 |
| Prefs | shared_preferences | 2.5 | device-local appearance only |
| Audio | record 7.x / just_audio 0.10 | at M1/M6 | record 7.1 needs Flutter 3.44 → use newest compatible |
| Secrets | flutter_secure_storage | 11.x | at M1 |
| Notifications | flutter_local_notifications 22.x, workmanager 0.10 | at M8 | |
| Markdown | markdown (Dart) 7.3 | at M3 | own renderer on top of its AST |
| Fonts | Vazirmatn, Inter, JetBrains Mono | latest releases, OFL | bundled, never fetched at runtime (privacy) |

`google_fonts` is deliberately **not** used: it fetches fonts over the network.
