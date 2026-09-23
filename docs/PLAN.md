# Daftar — Implementation Plan

Source of truth for scope: the build brief (sections referenced as §n). This document records how
we build it. Decisions that deviate from or refine the brief live in `docs/adr/`.

## 1. Architecture summary

```
┌──────────────────────────── Flutter app (app/) ─────────────────────────────┐
│ features/{capture,wiki,ask,voice,reflect,activity,settings}   design/ (tokens,│
│ Riverpod state · go_router · gen-l10n (en, fa) · custom components & icons     │
└───────────────▲──────────────────────────────────────────────────────────────┘
                │ flutter_rust_bridge v2 (daftar_bridge: thin API + DTOs)
┌───────────────┴────────────── Rust workspace (core/) ────────────────────────┐
│ daftar_core                                                                   │
│   layout · fsutil (atomic writes) · ids                                       │
│   repo      – repository init/open, SCHEMA.md template, config, devices       │
│   capture   – raw capture writer (immutable raw/ files, assets)               │
│   jobs      – durable SQLite job queue (retry, backoff, network wait)         │
│   sync      – git2: fetch → integrate (§5.4) → push; merge drivers; replay    │
│   index     – SQLite FTS5 + trigram, Persian normalisation, index.md gen      │
│   wiki      – page model: frontmatter, wikilinks, claims, sections            │
│   changeset – staged writes + validator (§6.4) → one commit + one ledger entry│
│   ledger    – op records, log/ appends, undo/revert/compensate                │
│   providers – openai_compatible, anthropic, gemini adapters; roles; usage     │
│   agent     – tool-calling loop, context budgeting, tools (§6.2)              │
│   ops       – ingest, query, lint, reflect, compensate                        │
│   mcp       – rmcp client, transports, OAuth 2.1 (PKCE, DCR, RFC 8707/9728)   │
│   voice     – VAD, sentence chunker, voice session state machine              │
│ daftar_cli  – same operations against a repo path (tests, evals, debugging)  │
│ prompts/    – versioned system prompts, embedded at compile time              │
└───────────────────────────────────────────────────────────────────────────────┘
         │ git (HTTPS/SSH)          │ HTTPS (providers)          │ MCP servers
```

Principles that shape the code:

* **Repository is truth; SQLite is a cache.** Anything in SQLite (search index, job queue mirror,
  embeddings) must be rebuildable from the repo plus local-only state (pending jobs, audio).
* **All AI writes go through the changeset pipeline.** The model never touches files; it calls tools
  that stage edits; code validates and commits.
* **One op = one commit = one ledger file.** Undo is `git revert` or a compensating op.
* **Core first, UI second.** Every operation is reachable from `daftar_cli` and tested there with the
  mock provider before the UI binds to it.

## 2. Milestones

| M | Scope (brief §14) | Exit criteria |
|---|---|---|
| M0 | Monorepo, CI, Flutter shell, FRB wiring, tokens + components, fa/en + RTL, CLI skeleton | Launches; live language/theme switching; goldens in CI |
| M1 | Onboarding (clone/init, PAT/SSH), raw capture, job queue, sync algorithm w/o replay | Scenarios 1 (no AI), 3, raw part of 2 |
| M2 | Providers, roles, STT, vision, router, agent loop, tools, validator, commits, ledger, log, index gen, replay | Scenarios 1, 2, 6, 9 |
| M3 | Renderer, backlinks, local graph, editor, Persian FTS, Wiki tab + desktop panes | Scenarios 7, 8; perf targets |
| M4 | Activity, diffs, undo, compensate, move, re-run, exclude, Review stack | Scenarios 4, 5 |
| M5 | Ask: streaming, citations, scopes, image input, save to wiki, story mode | Correct citations; no fabrication |
| M6 | Voice: VAD, STT→LLM→TTS streaming, barge-in, per-language voices | Scenario 11; latency |
| M7 | MCP: transports, all auth, per-device auth cards, policies, resources/prompts | Scenario 10 |
| M8 | Reflect + Lint, scheduler, patterns, notifications, wellbeing guardrail | Weekly review quality; lint verified quotes |
| M9 | Widgets, share, hotkey, a11y, perf, packaging, user guides | All scenarios green; release artifacts |

Order inside each milestone: core module + unit tests → CLI command + scenario test → bridge API →
UI → widget/golden tests → PROGRESS.md.

## 3. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| libgit2 on iOS/Android (OpenSSL, SSH) | Sync broken on mobile | `vendored-libgit2` + `vendored-openssl`; SSH via libssh2 vendored; CI builds mobile every push; fall back to HTTPS+PAT only if SSH fails on a platform (ADR). |
| Model output quality for ingest | Bad wiki | Strict tools + validator + repair rounds; eval harness; recorded fixtures. |
| Replay semantics after merges | Lost/duplicated facts | Ops are pure functions of (wiki, raw, schema); scenario 2/3 in CI with mock provider. |
| Background execution limits on iOS | Stale sync/reflect | Run on foreground; best-effort BGTask; never depend on background. |
| Echo cancellation on desktop | Barge-in false triggers | Platform voice-processing on mobile; on desktop require headset or push-to-talk fallback, documented. |
| Flutter/Dart pinned below latest packages | Missing fixes | ADR-0003; upgrade path is `flutter upgrade` + bump constraints. |
| Only Linux + Android verified locally | macOS/iOS/Windows regressions | CI builds every push; owner verifies on hardware at milestones. |
| Secrets leakage into repo | Credential exposure | Secure storage only; pre-commit secret scanner in changeset + human commit path. |

## 4. Open decisions (owner input welcome, defaults chosen)

1. **Bundle identifier / org**: `dev.daftar.daftar` placeholder until the owner provides a domain (needed for signing, universal links).
2. **Accent colour**: ink teal `#22505E` (light) / `#86B6C2` (dark) — ADR-0005.
3. **Default helpline data** for Iran and international — curated list shipped in M8; owner should review.
4. **Signing credentials** (Apple, Android keystore, Windows cert) — required for M9 release artefacts; CI builds unsigned until provided.

## 5. Environment notes

* Flutter 3.41.1 / Dart 3.11 locally (ADR-0003). Rust 1.96 stable.
* The development machine routes HTTP through a local proxy; `flutter test` needs
  `NO_PROXY=127.0.0.1,localhost` (see AGENTS.md).
