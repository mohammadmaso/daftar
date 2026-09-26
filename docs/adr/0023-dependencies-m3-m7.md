# ADR-0023: Dependencies added for M3–M8 (verified 2026-09-25)

* Status: accepted
* Date: 2026-09-25

| Need | Choice | Version | Notes |
|---|---|---|---|
| Markdown AST (app) | `markdown` (dart-lang) | 7.3.1 | Parser only; rendering is ours (§2). Newest version resolvable on Flutter 3.41.1 (ADR-0003). |
| MCP (core) | `rmcp` | 3.4.1 | Official Rust SDK, released 2026-09-23; see ADR-0021. |
| URLs (core) | `url` | 2.x | Already in the tree through reqwest; used for MCP/OAuth URL handling. |
| HTTP types (core) | `http` | 1.x | Needed to implement rmcp's `OAuthHttpClient` over our reqwest client. |
| Voice playback (app) | `audioplayers` | 6.7.1 | Plays each TTS sentence from bytes on all five platforms (GStreamer on Linux). Chosen over `just_audio`, which needs extra plugins for Linux/Windows. 6.8.1 exists but needs a newer Flutter (ADR-0003). |
| Screen awake (app) | `wakelock_plus` | 1.7.0 | Keeps the screen on in voice mode (§8.4). 1.8.0 needs a newer Flutter. |
| Open the system browser (app) | `url_launcher` | 6.3.2 | Desktop MCP OAuth: opens the consent page; the core listens on a loopback redirect (§10). flutter.dev plugin. |
| OAuth on phones (app) | `flutter_web_auth_2` | 5.1.0 | ASWebAuthenticationSession on iOS, Custom Tabs on Android, with the `daftar://oauth/callback` redirect. Never an embedded web view (§10). |
| Local notifications (app) | `flutter_local_notifications` | 22.3.1 | Neutral "your reflection is ready" notices (§4.7) on Android, iOS, macOS, Linux and Windows. Needs core-library desugaring on Android (`desugar_jdk_libs` 2.1.4) and `POST_NOTIFICATIONS`. |

The existing `record` 6.2.1 (ADR-0002) streams 16 kHz PCM with platform voice processing
(`voiceCommunication` source on Android, echo cancellation elsewhere) for voice mode.

All three app plugins were at their latest versions on pub.dev when added and are actively
maintained.

Not added on purpose:
* `workmanager` / background fetch for reflections: reflections are queued when the app opens,
  resumes, and every ten minutes while it stays open. A reflection that falls due while the app is
  closed runs the next time it opens; M9 revisits background work together with sync.
* `freezed` / `build_runner`: flutter_rust_bridge only needs them for Rust enums that carry data.
  The bridge uses plain structs with a kind enum instead, so bindings need no Dart code generation
  beyond FRB's own.
* A Markdown widget package: rendering needs wikilinks, callouts, inline fields, claim pills and
  per-paragraph bidi, which none of them offers well; we render the AST ourselves.
