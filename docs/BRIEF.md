# Build Brief: **Daftar** — a local-first, bilingual, voice-first personal Life OS built on the LLM-Wiki pattern

> You are a senior engineer building this product end to end. This document is your full
> specification. Work autonomously: when something is unspecified, make the most reasonable
> decision, record it as an ADR in `docs/adr/`, and keep going. Only stop to ask the owner when you
> are truly blocked (e.g. missing signing credentials). "Daftar" (دفتر, "notebook") is a codename;
> keep it in one constant so it can be renamed.

---

## 0. How to work

1. Read this entire document before writing code. Then write `docs/PLAN.md`: architecture summary,
   milestone breakdown (Section 14), risks, and open decisions. Keep `docs/PROGRESS.md` updated after
   every milestone: what works, what's stubbed, known bugs.
2. Create `AGENTS.md` at the root of the **code** repo describing build commands, architecture,
   conventions, and test commands for future agents.
3. Build milestone by milestone. A milestone is done only when its acceptance criteria pass on at
   least Linux desktop and Android. iOS and macOS must at least compile each milestone.
4. **Verify dependencies before adopting them.** Your training data may be stale. For every package
   or crate named here, check that it is maintained, check its latest version and platform support,
   and pick a better-maintained alternative if needed. Record the choice in an ADR.
5. Never fake functionality. No mocked "coming soon" screens in the shipped app. Unfinished features
   are hidden behind a compile-time flag.
6. Tests are part of the work: Rust unit and integration tests, Flutter widget and golden tests, and
   the scenario tests listed in Section 13.

---

## 1. Product vision

A personal assistant that **captures my life with near-zero friction** and **compiles it into a
living, interlinked Markdown wiki** that it maintains itself, following Andrej Karpathy's "LLM Wiki"
pattern.

### 1.1 The pattern

Instead of RAG over raw notes, the LLM incrementally builds and maintains a persistent wiki. When a
new source arrives, the LLM reads it and integrates it into existing pages:

- It updates entity and topic pages.
- It adds cross-links.
- It flags contradictions.
- It revises summaries.

Knowledge is compiled once and kept current. It is not re-derived on every question. There are
three layers:

- **Raw sources**: immutable, the source of truth.
- **The wiki**: LLM-owned Markdown.
- **The schema**: a user-editable document telling the LLM how the wiki works.

There are three operations: **Ingest**, **Query**, and **Lint**. There are two navigation files:

- `index.md`: a catalog.
- `log.md`: a chronological record.

### 1.2 What I do with it

- Hold a button, talk for 30 seconds in Persian or English (often mixed), and let go. It becomes a
  raw entry, gets transcribed, and gets filed into the right place. It updates my journal, my people
  pages, my concerns, my health profile, my story notes, and so on, with links between them.
- Snap a photo (a prescription, a whiteboard, a book page, a meal, a lab result). A vision model
  reads it and it is ingested the same way.
- Ask questions by text chat or by a **fully hands-free voice conversation** ("What did the doctor
  say about my vitamin D last spring?", "What have I been worried about this month?", "Help me
  continue chapter 4 — what does Sara know at this point?").
- Get proactive reflections: daily and weekly reviews, recurring patterns, ideas related to my
  concerns. Every claim is backed by links to the entries it came from.
- Connect external tools through MCP servers with any auth scheme.
- Sync everything across Linux, Android, iOS, macOS, and Windows through **one private Git
  repository that I own. No server, no backend, no cloud service of yours.**

### 1.3 Product principles (non-negotiable)

- **Few features, each excellent.** Everything maps to one of four loops: **Capture → Compile → Ask
  → Reflect**. Anything else is out of scope.
- **Local-first and offline-first.** Every feature that does not strictly need an AI API works
  fully offline: capture, browse, read, edit, search, audit, and undo. AI jobs queue while offline
  and run when the network returns. Git sync is opportunistic.
- **No server.** Devices only talk to (a) the user's Git remote, (b) the user-configured AI
  providers, and (c) user-configured MCP servers.
- **The user owns the data.** The repository is plain Markdown plus plain files and must be fully
  usable in Obsidian. The app must also work when the user edits files outside it.
- **Everything the AI does is auditable and reversible** (Section 7).
- **Bilingual to the core:** Persian (RTL) and English (LTR) are equal first-class citizens,
  including mixed-direction text in the same paragraph.
- **It must not look AI-made** (Section 11). It should feel like a crafted, premium, native app.

### 1.4 Non-goals

- Local or on-device LLM providers.
- A hosted web version.
- Multi-user collaboration or sharing.
- End-to-end encryption of the repo (a private repo is sufficient).
- Exposing the app itself as an MCP server.
- A global "everything" graph view.
- Plugins.
- Tasks and to-do management.
- Calendar.

---

## 2. Tech stack (decided; verify versions)

| Layer | Choice | Why |
|---|---|---|
| UI (all platforms) | **Flutter** (stable channel, Dart 3, Impeller) | True native apps for Android, iOS, Linux, macOS, and Windows from one codebase. Custom-drawn UI gives full design control and consistent RTL/bidi. |
| Core logic | **Rust** crate `daftar_core`, bound with **flutter_rust_bridge v2** | One implementation of sync, storage, indexing, the agent loop, providers, and MCP across all platforms. Predictable performance. |
| Git | `git2` (libgit2) with `vendored-libgit2`, `vendored-openssl`, HTTPS + SSH | Full clone/fetch/merge/push on every platform, including mobile. Evaluate `gix` only if push and merge support are complete; record an ADR. |
| Local DB (cache only, rebuildable) | SQLite via `rusqlite` (bundled, FTS5) | Search index, job queue, op journal mirror, embedding cache. **Never the source of truth.** |
| Async / HTTP | `tokio`, `reqwest` with rustls, SSE streaming | Streaming LLM responses and audio. |
| MCP client | Official Rust SDK `rmcp` (client, stdio, Streamable HTTP, auth features) | Standards-compliant MCP. |
| OAuth | `oauth2` crate plus custom RFC 8414/9728/7591 discovery | MCP authorization (Section 10). |
| State management | Riverpod | |
| Routing | go_router | |
| Localization | Flutter `gen-l10n` (ARB files: `en`, `fa`) | |
| Markdown | `markdown` (Dart) parser plus **our own renderer** with custom syntaxes | We need wikilinks, callouts, block IDs, inline fields, and perfect bidi. Don't depend on an unmaintained renderer. |
| Editor | Custom source editor with syntax highlighting and live bidi (based on a maintained code-editor package, or a custom `EditableText` wrapper) | |
| Audio capture and playback | `record`, `just_audio` (verify) | |
| Voice activity detection | Silero VAD (ONNX) via Rust `ort` or a maintained Flutter package; energy-based fallback | For hands-free voice mode. This is not an AI provider and runs on-device. |
| Secrets | `flutter_secure_storage` (Keychain / Keystore / libsecret / DPAPI) | API keys, Git credentials, and OAuth tokens never touch the repo. |
| Notifications and background | `flutter_local_notifications`; `workmanager` (Android), BGTaskScheduler (iOS) — best effort | |
| Fonts | **Vazirmatn** (Persian) and **Inter** (Latin), bundled; JetBrains Mono for code | |

**Packaging targets:**

- Linux: AppImage (primary), .deb, Flatpak.
- Android: APK and AAB.
- iOS: IPA.
- macOS: signed DMG.
- Windows: MSIX.

Set up CI (GitHub Actions) that builds all of them and runs the tests.

**Code repo layout:**

```
/app            Flutter app (lib/features/{capture,wiki,ask,voice,reflect,activity,settings}, lib/design/)
/core           Rust workspace: daftar_core (lib), daftar_cli (dev/debug CLI exposing the same ops)
/core/fixtures  sample knowledge repos, recorded provider responses
/docs           PLAN.md, PROGRESS.md, adr/, design/
AGENTS.md
```

A **`daftar_cli`** binary exposing ingest, query, lint, sync, and search against a repo path is
required. It makes the core testable without the UI.

---

## 3. The knowledge repository (the user's Git repo)

The app works on one Git repo per "library" (support one library in the UI now; keep the code
multi-library-ready). The layout is **Obsidian-compatible**:

```
/SCHEMA.md                    user-editable wiki rules the in-app agent must follow (seeded from a template, co-evolves)
/.daftar/
    config.json               non-secret settings shared across devices: vaults, schema version, preferences
    devices/<device-id>.json  device name, platform, last seen
    ledger/<yyyy>/<mm>/<op-id>.json   one file per AI operation (Section 7); never edited after creation
/.gitattributes               log files → merge=union; *.md text eol=lf
/raw/<yyyy>/<mm>/<dd>/<ts>-<device>-<ulid>.md       one immutable file per capture
/raw/assets/<yyyy>/<mm>/<ulid>.<ext>                images (compressed), optional audio
/vaults/<vault>/index.md      GENERATED by code, not the LLM (Section 4.4)
/vaults/<vault>/**.md         LLM-maintained wiki pages
/log/<yyyy>-<mm>.md           human-readable chronological log, append-only, union-merged
```

### 3.1 Raw captures (immutable)

Every capture is its own file with a globally unique name, so **raw/ can never conflict**.
Frontmatter:

```yaml
id: 01JABC...            # ULID
kind: voice | text | photo | voice-conversation | chat-answer | import
captured_at: 2026-09-23T14:15:02+02:00
device: pixel-8
lang: [fa, en]           # detected
vault_hint: null         # set if user pinned a vault before capturing
assets: [raw/assets/2026/09/01JABD....webp]
transcript_model: <provider/model>   # for voice
status: pending | ingested | excluded
```

The body holds the transcript, typed text, or the vision description plus OCR. `status` is the only
field the app may later change (a tiny, conflict-safe edit). The content is never changed.

**Assets policy:**

- Images: resize to ≤ 1600 px on the long edge, WebP or JPEG around q80.
- Audio: **not committed by default** (it bloats Git history forever). Keep it locally for 14 days
  so it can be re-transcribed. A setting allows committing Opus at 16 kbps.
- No Git LFS.

### 3.2 Vaults

Vaults are top-level folders under `/vaults/`. They form one repo and one Obsidian vault, so
cross-vault links work. Seed these defaults (user can add, rename, archive):

| Vault | Purpose | Example page types |
|---|---|---|
| `life` | journal, people, places, goals, concerns, ideas | `journal/2026/2026-09-23.md`, `people/sara.md`, `concerns/career-direction.md` |
| `health` | medical profile | `profile.md`, `conditions/`, `medications/`, `labs/`, `visits/`, `symptoms-log.md` |
| `mind` | psychological self-model, moods, patterns, values | `profile.md`, `patterns/`, `moods/2026-09.md` |
| `work` | projects, learning, professional notes | `projects/`, `topics/` |
| `stories` | fiction; **each story is an isolated sub-workspace** | `stories/<story>/characters/`, `places/`, `timeline.md`, `threads/`, `chapters/` |

Rules:

- A single capture may touch several vaults. For example, a journal entry mentioning a headache
  updates `life/journal` and proposes a symptom in `health`.
- **Isolation:** content from `stories` must never produce claims about the user in `life`,
  `health`, or `mind`, and vice versa. Enforce this in the write validator (Section 6.4), not only
  in the prompt.

### 3.3 Wiki page format

Filenames are **ASCII kebab-case slugs** (the LLM proposes one; the code sanitizes it and normalizes
Unicode to NFC). This avoids NFD/NFC Git problems on macOS and path issues on Windows and Android.
Human-facing titles live in the frontmatter:

```yaml
---
id: 01JAB...
type: person | topic | concern | condition | medication | lab | journal-day | profile | pattern | idea | character | place | thread | answer | summary
vault: health
title: { en: "Vitamin D deficiency", fa: "کمبود ویتامین D" }
aliases: ["vitamin d", "ویتامین دی"]
summary: "One line, in the page's primary language."   # feeds the generated index
sources: [01JABC..., 01JABD...]                      # raw ids
created: 2026-09-23
updated: 2026-09-23
status: active | stale | archived
---
```

- Links use Obsidian wikilinks: `[[vitamin-d-deficiency|کمبود ویتامین D]]`. Every mention of a
  known entity should be linked the first time it appears in a section.
- Source citations are inline: `([[raw/2026/09/23/…|voice · 23 Sep]])`. Every factual sentence the
  LLM adds must trace to at least one source. The validator checks that each changed section cites
  something.
- The body language follows the source language by default. Mixed Persian and English is fine.
  Titles and aliases are **always bilingual**, so search and linking work in both languages.

### 3.4 Claims about the user (the "unverified until confirmed" rule)

Anything the LLM asserts **about the user** in `health`, `mind`, or the `life` profile and concern
pages is a **claim**. Claims are written as list items with Dataview-compatible inline fields and a
block ID:

```markdown
- Takes vitamin D 50,000 IU weekly (status:: confirmed) (src:: [[raw/…|voice · 3 Mar]]) ^c-01JAB9
- Sleep quality tends to drop on days after late coffee (status:: proposed) (confidence:: medium) (src:: [[raw/…]], [[raw/…]], [[raw/…]]) ^c-01JAC2
```

- `proposed` claims appear in the **Review** queue (Section 8.5). Swiping confirms, rejects, or
  edits them.
- **Directly stated** facts ("I started taking X") may be auto-confirmed. **Inferred** facts
  (patterns, interpretations, anything the user did not literally say) are always `proposed`.
- Rejected claims are removed and recorded in the op ledger, so the LLM doesn't re-propose the same
  thing from the same evidence.
- Contradictions never overwrite silently. The old claim becomes `(status:: superseded)` with a link
  to the new one, and the change appears in Review.

### 3.5 SCHEMA.md

Seed `SCHEMA.md` from a template the app ships (bilingual). It contains:

- The vault purposes.
- Page types and their required sections.
- Linking and citation rules.
- The claim rules.
- Writing style ("concise, factual, no filler, no motivational fluff; the user's own words are
  preserved in quotes where meaningful").
- Journal conventions.
- Story rules.

The in-app agent **loads SCHEMA.md into every ingest, query, and lint prompt**. The user can edit it
in-app. The agent may *propose* schema changes through Review but may never edit it directly.

---

## 4. Core operations

All operations run in `daftar_core` as **jobs** in a persistent SQLite queue. Jobs survive app
restarts, retry with backoff, and wait for the network. Every AI write goes through the **changeset
pipeline** (Section 6.4) and produces exactly **one Git commit plus one ledger entry**.

### 4.1 Capture (offline, instant)

- Voice, text, photo, or share-sheet import (text, URL, image, PDF) writes the raw file
  **immediately** and locally. The UI confirms in under 100 ms with haptic feedback.
- It enqueues: `transcribe` (voice), then `describe` (photo, via the vision model: description plus
  OCR plus detected document type), then `ingest`.
- The user may pin a vault before capturing (long-press the capture button to pick one). Otherwise
  routing is automatic.

### 4.2 Ingest (the heart of the product)

For one raw item:

1. **Route.** A small, fast model (the `router` role) reads the item plus the vault list plus the
   vault index summaries. It returns structured JSON:

   ```json
   {"targets": [{"vault": "life", "reason": "...", "confidence": 0.92},
                {"vault": "health", "reason": "...", "confidence": 0.71}],
    "is_fiction": false, "story": null, "lang": ["fa"]}
   ```

   - Below the confidence threshold (default 0.6), ingest into the best guess **and** add a "Was
     this right?" item to Review.
   - If `vault_hint` is set, it wins.
2. **Plan.** The `ingest` model gets:
   - SCHEMA.md,
   - the target vaults' generated `index.md`,
   - search hits (Section 4.5),
   - the full pages it chooses to open (via tools).

   It decides which pages to create or update. Typically 1–15 pages: the journal day, entities,
   concerns, profile claims, and back-links.
3. **Write** through the tool API only (Section 6.2). It cannot write files directly.
4. **Validate, commit, log.** The pipeline validates the changeset (Section 6.4), commits it,
   appends to the monthly log, marks the raw item `ingested`, writes the ledger entry, and
   regenerates the affected `index.md` files.
5. Show a subtle, non-blocking result in the timeline: "Filed to Life · Health — 4 pages updated, 1
   claim to review". Tapping it opens the Activity detail (Section 7).

**Journal behavior:** each capture that is personal gets a timestamped section in
`life/journal/<yyyy>/<date>.md`. The section contains the gist in the user's language plus a link to
the raw item. Extracted entities are linked from it. The day page gets an end-of-day summary during
Reflect.

### 4.3 Query (Ask)

- Flow: read the relevant `index.md` files, then search, then open pages, optionally call MCP tools,
  then answer.
- Answers are streamed, in the user's language, and **cite wiki pages and raw sources as tappable
  links**.
- If the wiki doesn't contain the answer, say so plainly. Never invent personal facts.
- Every answer has a **"Save to wiki"** action. It files the answer as an `answer`-type page (or
  merges it into an existing page) through the normal ingest pipeline, so explorations compound.
- Story mode: when a story workspace is active, retrieval is restricted to that story and the
  assistant acts as a co-writer. It keeps track of continuity (who knows what, when), suggests
  options, and drafts only when asked. The user's prose is never overwritten; drafts are saved as
  separate files.

### 4.4 Index and log (deterministic, conflict-free)

- `vaults/<vault>/index.md` is **generated by code** from page frontmatter (title, summary, type,
  updated, source count), grouped by type. It is never hand-merged; on any conflict it is
  regenerated. The LLM only affects it through each page's `summary` field.
- `log/<yyyy>-<mm>.md` is append-only. Entries look like
  `## [2026-09-23 14:15] ingest | life, health | "headache after bad sleep" (op 01JAB…)`.
  `.gitattributes` sets `merge=union` so appends from different devices never conflict.

### 4.5 Search

- SQLite FTS5 over titles, aliases, summaries, and bodies, with a **Persian-aware normalization
  layer** applied at both index and query time:
  - Arabic ي/ك → Persian ی/ک
  - Remove tatweel and diacritics (harakat)
  - Normalize ZWNJ (index both the joined and split forms)
  - Persian and Arabic digits → ASCII
  - Unify ه/ة and hamza variants
  - Latin case folding and accent folding
- Add a trigram index as a fallback for partial matches.
- Optional: remote embeddings (the `embedding` role, if configured) stored in the local cache only,
  used for hybrid ranking. The app must work well **without** embeddings: index files plus FTS are
  the default path.
- The index is a rebuildable cache. It is rebuilt incrementally after every pull or commit, and
  fully on demand.

### 4.6 Lint (maintenance)

Runs weekly, on demand, or after N ingests.

Deterministic checks (code):

- broken links
- orphan pages
- missing or invalid frontmatter
- duplicate slugs or aliases
- oversized pages
- raw items stuck in `pending`

LLM checks:

- contradictions between pages
- stale claims superseded by newer sources
- important entities mentioned but missing a page
- pages that should be merged or split

LLM findings must quote the exact conflicting lines. **Code verifies each quote exists before
surfacing it**; unverifiable findings are dropped. Findings go to Review as proposed fixes. Nothing
is applied without approval, except trivial deterministic fixes (index regeneration, link
normalization).

### 4.7 Reflect (proactive, gentle, evidence-based)

Scheduled locally. Because there is no server, it runs at app open or in best-effort background
tasks when due.

- **Daily** (evening, configurable): a day summary on the journal page, plus a local notification
  such as "Your day, filed", with at most 3 lines.
- **Weekly**: `life/reviews/<yyyy>-W<ww>.md` covering:
  - what happened,
  - progress on concerns and goals,
  - recurring themes,
  - mood and energy trends from `mind`,
  - health notes,
  - 2–3 concrete ideas or suggestions tied to current concerns,
  - open questions for the user.
- **Pattern detection:** proposes `pattern` claims in `mind` or `health` **only with ≥ 3 linked
  supporting entries**. Always `proposed`, always with evidence links.

**Wellbeing guardrail (must implement):**

- Reflections describe observations; they never diagnose or label ("you are depressed").
- If recent entries contain signs of crisis (self-harm, suicidal thoughts, danger), Reflect and Ask
  respond with warmth and prominently surface a "Talk to someone" card. It points to professional
  help and a crisis line. The user sets their country and preferred helpline in settings, with
  sensible defaults for Iran and international.
- This content is never turned into flippant notifications.
- Health answers include "for decisions, confirm with your doctor" only when the answer involves
  treatment or medication decisions. Not everywhere.

---

## 5. Git sync (serverless, conflict-proof by design)

Implement in `daftar_core::sync` with libgit2.

### 5.1 Setup

- **Onboarding:** "Connect a private repository".
  - Paste the HTTPS URL plus a personal access token, **or** generate an ed25519 SSH key in-app and
    show the public key with a copy button and instructions for GitHub, GitLab, and Gitea.
  - Clone. If the repo is empty, initialize the structure and the SCHEMA.md template.
  - Name the device.
  - Credentials go to secure storage.
- Branch: `main` by default (configurable).

### 5.2 Sync loop

Triggers:

- app foreground
- after each commit (debounced 10 s)
- every N minutes while in the foreground
- background task where allowed
- the manual pull-to-refresh gesture

Steps: fetch → integrate → push. If there is no network, do nothing and show nothing alarming. A
tiny sync indicator shows `synced` / `n local changes` / `offline` / `needs attention`.

### 5.3 Why conflicts are rare

- `raw/` and `.daftar/ledger/` use unique filenames, so they never conflict.
- `log/` uses union merge.
- `index.md` files are generated and are regenerated after a merge.
- **Ingest ops are replayable.** Raw is the source of truth, so an op is a function of (wiki state,
  raw item, schema).

### 5.4 Integration algorithm

1. **Fetch.** If local has no unpushed commits, fast-forward.
2. If both sides diverged, try a normal 3-way merge (libgit2) with the `.gitattributes` drivers.
3. If a **wiki page** conflicts and the local conflicting commit is an AI op, **discard that local
   op commit** and re-enqueue the op for **replay on top of the merged state**. The raw item and the
   router decision are reused; the LLM re-applies the ingest to the current pages. The ledger
   records `replayed_from`.
4. If the conflict involves **human edits** on both sides, write a proper 3-way text merge where
   possible. For true conflicts, keep both versions inside a clearly marked callout
   (`> [!conflict] From <device> …`) and create a Review item to resolve. Never lose text. Never
   leave Git conflict markers in files.
5. **Double-ingest guard.** Before ingesting, check the ledger and the raw `status` after fetching.
   If another device already ingested the item, skip it. Two devices ingesting the same item
   concurrently is resolved by keeping the op with the earlier ULID and discarding and replaying
   nothing for the other.
6. **Push.** On non-fast-forward, loop back to step 1, with a maximum of 3 attempts, then surface
   `needs attention` with a clear explanation and a "Retry" button.

### 5.5 Other sync details

- **External edits** (Obsidian on desktop, or editing on GitHub) are picked up on the next pull. The
  watcher re-indexes changed files. Pages edited by the human are marked in the ledger so the LLM
  treats human text with respect: it may append and link, but must not rewrite human-authored
  paragraphs without a Review approval. Track authorship per section via the ledger plus Git blame;
  don't add clutter to files.
- **Commit messages** are structured and human-readable, with trailers:

  ```
  ingest: voice note → life, health (4 pages)

  Op-Id: 01JAB...
  Op-Type: ingest
  Source: raw/2026/09/23/...
  Vaults: life, health
  Device: pixel-8
  ```

---

## 6. The in-app agent

### 6.1 Loop

A tool-calling agent loop in Rust with:

- streaming,
- parallel tool calls where the provider supports them,
- a max-steps budget per op type,
- token budgeting that trims context (drops the oldest tool results and **replaces earlier reads of
  a file that was since edited** so stale copies don't pile up),
- cancellation,
- prompt caching where the provider supports it.

The same loop serves Ingest, Query, Lint, Reflect, and Voice, with different system prompts, tool
sets, and model roles.

### 6.2 Internal tools (exposed to the model)

| Tool | Notes |
|---|---|
| `index_read(vault?)` | the generated index(es) |
| `search(query, vault?, types?, limit)` | FTS + optional hybrid; returns path, title, summary, snippet |
| `page_read(path, range?)` | returns **line-numbered** content plus a content hash |
| `page_create(path_slug, frontmatter, body)` | code fills `id`, `created`, `updated`, `vault`; slug sanitized |
| `page_edit(path, base_hash, edits[])` | **line-based** edits: `replace_lines`, `insert_after_line`, `append_to_section(heading)`, `add_frontmatter_values`. Rejected if the hash is stale, which forces a re-read. No fuzzy string search-and-replace. |
| `claim_propose(page, text, sources[], kind: stated\|inferred, confidence)` | code formats the claim line and sets status per Section 3.4 |
| `claim_supersede(claim_id, new_text, sources[])` | |
| `raw_read(id)` / `asset_view(id)` | asset_view passes the image to a vision-capable model |
| `link_suggest(text)` | deterministic: finds known titles and aliases in text to link |
| `review_add(kind, payload)` | asks the user something asynchronously via the Review queue |
| MCP tools | namespaced `mcp.<server>.<tool>`, subject to the per-server approval policy |

### 6.3 Model roles

Each role maps to a provider plus model plus parameters (Section 9):

- `router`
- `ingest`
- `chat` (Ask)
- `voice` (conversational, low latency)
- `vision`
- `reflect`
- `lint`
- `stt`
- `tts`
- `embedding` (optional)

The user can point several roles at the same model.

### 6.4 Changeset pipeline (enforced in code, not trusted to the model)

All writes in an op are staged in memory. Before committing, the validator runs these checks:

- Frontmatter schema is valid; required fields are filled by code (dates, ids, vault).
- No writes to `raw/`, `.daftar/`, `SCHEMA.md`, or generated `index.md`.
- Every wikilink resolves, or a stub page was created in the same changeset (stubs allowed, with
  `status: stub`).
- Every changed section cites a source.
- **Vault isolation:** an op whose router said `is_fiction: true` cannot write outside
  `stories/<story>`. Non-fiction ops cannot write into `stories/`.
- Claims in `health`, `mind`, and the life profile use claim syntax, and inferred claims are
  `proposed`.
- Size limits are respected (warn above 400 lines per page; suggest a split via lint).
- No human-authored paragraph is modified without an approved Review item.

If validation fails, feed the errors back to the model for up to 2 repair rounds. If it still fails,
abort the op, keep the raw item `pending`, and surface it in Activity as failed with a readable
reason.

---

## 7. Audit, undo, and correction (first-class)

Every AI op has:

- a ledger entry `.daftar/ledger/…/<op-id>.json`: op type, raw source, router decision with reasons
  and confidences, model and provider, pages created and updated, claims added, token usage,
  timestamps, device, commit SHA, and `replayed_from` / `reverted_by`;
- exactly one commit.

The **Activity** screen is a reverse-chronological list of ops. Each op opens a detail view with:

- a human summary ("Filed your voice note to **Life** and **Health**: updated *Journal · 23 Sep*,
  *Sara*, proposed 1 claim in *Health profile*"),
- the reason the vault was chosen,
- a readable diff per page (rendered, with additions and removals highlighted, plus a raw diff
  toggle).

Actions:

- **Undo:** `git revert` of the op commit. If later commits make the revert conflict, fall back to a
  **compensating op**. The LLM receives the original op's diff plus the current pages and must
  remove only information derived solely from that source, then commit as `revert-op`. The raw item
  is marked `excluded` (it stays in raw/ and can be re-included).
- **Move to vault…:** undo, then re-ingest with a forced vault.
- **Re-run with note:** the user adds a correction ("this is about my story, not me" or "Sara is my
  cousin, not my colleague"), then undo, then re-ingest with the note attached. If the note is a
  durable fact, a confirmed claim is also added.
- **Exclude source:** undo, and never ingest this raw item again.

Undo of an undo works. Nothing is ever destroyed; Git history is the ultimate audit trail.

---

## 8. Screens and UX

### 8.1 Mobile (Android and iOS): three destinations, one gesture

Bottom bar: **Today**, **Wiki**, **Ask**. Settings is reached from the avatar or monogram in the top
corner. Review and Activity are reached from a small badge on Today.

- **Today**
  - Most of the screen is a calm timeline of today's captures, each with its filing status.
  - At the bottom center is the **capture button**:
    - hold to record voice, release to save;
    - slide up to lock hands-free recording;
    - tap for a text sheet;
    - the camera icon beside it opens capture;
    - long-press opens the vault picker.
  - While recording, show a live waveform, the timer, and a slide-to-cancel gesture.
  - At the top: the date, a one-line day summary once available, and a Review badge when items are
    waiting.
- **Wiki**
  - Vault switcher chips.
  - Search field (bilingual, instant).
  - Recently updated pages.
  - Pinned pages (Health profile, Mind profile, Concerns, active story).
  - The page reader (Section 8.3).
- **Ask**
  - A chat thread with streaming answers and citations.
  - A prominent **voice** button opens Voice mode (Section 8.4).
  - Attach a photo.
  - An optional scope chip: all / a vault / a story.
- **Share-sheet extension** on both platforms: sharing into Daftar creates a raw capture.
- **Home-screen widget and quick action:** "Record" opens straight into recording.

### 8.2 Desktop (Linux, macOS, Windows)

Three panes:

- **Left:** vaults and page tree, Today, Review, Activity.
- **Center:** the page reader and editor.
- **Right:** a collapsible Ask panel that is aware of the current page.

Global hotkey (where the platform allows) for quick capture, including voice. Full keyboard
navigation and a command palette (Ctrl/Cmd+K) for search, open, capture, and ask.

### 8.3 Page reader and editor

Beautiful Markdown rendering:

- headings and typographic rhythm
- wikilinks (tappable, hover preview on desktop)
- callouts (`> [!note]`, including `conflict`)
- tables (horizontally scrollable)
- task lists
- code blocks with highlighting
- images from assets
- footnotes
- inline fields rendered as subtle chips
- claim lines with a status pill (confirmed / proposed / superseded)
- frontmatter shown as a compact property header (type, vault, updated, sources count), not raw YAML

Also:

- A **Backlinks** section at the bottom.
- A small **local graph** (depth 1–2) in a bottom sheet or side panel, animated and minimal.
- **Per-paragraph direction detection** (first strong character), with correct mixed
  Persian/English inline bidi, Persian punctuation, and ZWNJ rendering.
- Edit mode is the source editor with highlighting and bidi-aware cursor movement. Saving creates a
  human commit (`edit: <page>`).

### 8.4 Voice mode (hands-free conversation)

- A full-screen, minimal view: one softly animated shape that reacts to input and output amplitude,
  the current state (listening / thinking / speaking), live captions (toggleable), and two controls:
  mute and end.
- Pipeline:
  1. VAD detects end of speech (tunable silence threshold, default 700 ms).
  2. STT.
  3. The `voice`-role LLM, with the same tools as Ask plus MCP, streams tokens.
  4. Sentence-chunked streaming TTS starts speaking as soon as the first sentence is ready.
  5. Return to listening.
- **Barge-in:** if the user starts speaking while TTS plays, stop playback immediately and listen.
  Use echo cancellation (platform voice-processing audio session: `AVAudioSession` voiceChat mode,
  Android `VOICE_COMMUNICATION` source, WebRTC-AEC-style on desktop if feasible).
- Mixed-language speech works. TTS voice selection is per language (a Persian voice and an English
  voice), chosen automatically per sentence based on script.
- Target latency from end of speech to first audio: < 1.5 s with fast providers. Show a thinking
  state immediately.
- The conversation transcript is saved as a `voice-conversation` raw item and ingested afterwards
  (the user can disable this per session). A voice command like "remember that…" triggers an
  immediate capture.
- Architect a `RealtimeVoiceProvider` interface so a speech-to-speech realtime API can be added
  later. Ship the pipeline version first.
- Keep the screen awake. Support Bluetooth headsets. On mobile, keep the session alive with a
  foreground service (Android) or the background audio mode (iOS) while voice mode is active.

### 8.5 Review queue

One swipeable card stack. Card types:

- proposed claims (confirm / edit / reject),
- low-confidence routing ("Filed to Health — right?", with vault chips to move),
- lint findings with a proposed fix (apply / dismiss),
- sync conflicts to resolve,
- schema change proposals,
- agent questions (`review_add`).

It must feel quick and satisfying: 30 seconds a day, not a chore.

### 8.6 Settings (deliberately short)

- **Providers**
- **Models**: role → provider + model
- **Voice**: STT/TTS voices, VAD sensitivity, captions, save transcripts
- **Repository**: remote, credentials, branch, device name, sync status and log, "Rebuild index"
- **Vaults**
- **MCP servers**
- **Reflect**: times, notifications, helpline country
- **Appearance**: language fa/en/system, theme light/dark/system, text size
- **Storage**: keep audio, image quality
- **About**

---

## 9. AI providers (bring your own key)

A provider config (stored in `config.json` **without secrets**; secrets in secure storage keyed by
provider id):

```json
{ "id": "p1", "name": "My OpenRouter", "kind": "openai_compatible",
  "base_url": "https://…/v1", "extra_headers": {"X-Title": "Daftar"}, "timeout_s": 60 }
```

`kind` adapters to implement:

1. **`openai_compatible`**
   - Chat Completions with tools, streaming, and vision content parts.
   - `/audio/transcriptions` for STT.
   - `/audio/speech` for TTS (streaming if supported).
   - `/embeddings`.
   - `/models` for listing.
   - This covers OpenAI, OpenRouter, Groq, Together, DeepSeek, Azure-style endpoints, LiteLLM
     proxies, and more.
2. **`anthropic`**: Messages API with tools, streaming, vision, and prompt caching.
3. **`gemini`**: generateContent with tools and vision.

Each **role** config:

```json
{ "role": "stt", "provider": "p1", "model": "whisper-large-v3", "params": {"language": "auto"} }
```

Requirements:

- A "Test" button per role that performs a real minimal call and reports latency.
- A model dropdown populated from `/models` when available, with free-text entry always allowed.
- Validate capability mismatches (e.g. a non-vision model assigned to the vision role) with a clear
  warning.
- Rate-limit and retry with backoff.
- Surface provider errors in human language; never show raw stack traces.
- Track token usage and approximate cost per op (shown in Activity details). Per-model prices are
  user-editable.

---

## 10. MCP client (any transport, any auth)

- **Transports:**
  - **Streamable HTTP** (primary), and legacy **HTTP+SSE**, on all platforms.
  - **stdio** on desktop only (command, args, env, working dir). Hide stdio on mobile with a short
    explanation.
- **Auth methods:**
  - None.
  - Static bearer token.
  - API key in a custom header or query parameter.
  - Arbitrary custom headers.
  - **OAuth 2.1 authorization code + PKCE** per the MCP authorization spec:
    - protected-resource metadata discovery (RFC 9728) from the `WWW-Authenticate` header;
    - authorization-server metadata (RFC 8414 / OIDC discovery);
    - **dynamic client registration** (RFC 7591) when supported, otherwise a manually entered
      client id and secret;
    - the `resource` parameter (RFC 8707);
    - token refresh.
  - OAuth **client credentials**.
- **OAuth redirects:**
  - Desktop: loopback `http://127.0.0.1:<random>/callback`.
  - Mobile: a custom scheme `daftar://oauth/callback` plus universal or app links.
  - Use the system browser (ASWebAuthenticationSession / Custom Tabs), never an embedded webview.
- Tokens live in secure storage. **MCP server configs sync via the repo without secrets.** Each
  device re-authenticates once, prompted by a card: "Connect <server> on this device".
- **Per-server tool policy:** `ask every time` (default for tools not annotated read-only),
  `auto for read-only`, or `always allow`. Approval cards appear inline in chat and as a spoken
  confirmation in voice mode.
- The server list shows status (connected / needs auth / error), tool count, and last error. Support
  tools, resources (readable into context), and prompts (as slash commands in Ask).
- MCP tool results can be filed to the wiki through "Save to wiki", like any answer.

---

## 11. Visual design (must not look AI-generated)

**Direction: "calm paper, precise ink".** Quiet, typographic, and tactile, like a beautifully made
notebook app. It is not a chatbot, not a dashboard, and not a template.

Hard rules:

- **No** sparkle or star "AI" icons, no purple-blue gradients, no glassmorphism blobs, no emoji in
  the UI, no "✨ Generated by AI" labels, no robot avatars, no chat bubbles with tails, no generic
  Material look (build custom components on a token system), and no default Flutter blue.
- **Copy** is short, human, and specific ("Filed to Life and Health", not "Your content has been
  successfully processed!"). The assistant has no name or persona in the UI; it simply speaks.
- **Design tokens** live in `lib/design/tokens.dart`:
  - a 4/8 pt spacing grid;
  - a type scale tuned separately for Vazirmatn and Inter (Persian needs roughly 1.1–1.15× size and
    more line height, about 1.8 for body text);
  - **one** accent color (a deep ink tone such as muted indigo or ink teal; the designer's choice,
    documented);
  - a warm off-white paper background for light theme and a true deep charcoal (not pure black) for
    dark theme;
  - 1 px hairlines instead of heavy shadows;
  - corner radius 10–14.
- **Motion:** 150–250 ms with ease-out or spring; meaningful only (capture confirmation, filing
  status). Haptics on capture start/stop and on Review swipes. Respect reduced motion.
- **RTL:** layouts mirror fully in Persian UI. Icons with direction (back, chevrons) mirror; media
  and waveform icons don't. The UI language (fa/en) is independent from content direction, which is
  always per paragraph.
- **Accessibility:** WCAG AA contrast, screen-reader labels in both languages, dynamic type up to
  200 % without broken layouts, touch targets ≥ 44 pt.
- Produce `docs/design/` with a mini style guide (screenshots of every component in light, dark, en,
  and fa) and **golden tests** for key screens in all four combinations.
- Custom app icon: a simple, elegant mark (e.g. an abstract folded page or a single ink stroke). Not
  a brain, not a robot, not a sparkle.

---

## 12. Quality attributes

- **Performance:**
  - Cold start under 1.5 s on a mid-range Android phone.
  - Capture-to-saved under 100 ms.
  - Wiki search under 50 ms at 5,000 pages.
  - Smooth 60/120 fps scrolling on long pages.
  - The page tree and indexes load lazily.
- **Scale target:** 20k raw items, 5k wiki pages, and a repo size of 1 GB+ without degrading.
- **Reliability:** capture never fails because of network or AI errors. Jobs are durable. Every
  write is atomic (write to temp, fsync, rename). Crash recovery is idempotent.
- **Security:**
  - Secrets only in platform secure storage.
  - Never logged.
  - Redacted in diagnostics.
  - A pre-commit guard blocks committing anything that looks like an API key or token (warn the user
    and offer to redact).
  - TLS only, except loopback OAuth.
- **Privacy:** no telemetry, no analytics, and no network calls other than the user's remote,
  providers, and MCP servers. The app has a local-only diagnostics log that the user can export.
- **Portability:** the repo must open in Obsidian with working links, properties, and Dataview
  inline fields, and must stay readable forever without the app.

---

## 13. Required scenario tests

Automated, using `daftar_cli` and fixture repos plus a **mock provider** that replays recorded
responses:

1. **Offline capture:** 20 captures with no network, then reconnect. All are transcribed, ingested,
   committed, and pushed in order, with no duplicates.
2. **Two-device divergence:** device A and device B each ingest different captures touching the same
   person page while offline, then both sync. The final state contains both facts, there are no
   conflict markers, one op shows `replayed_from`, and the indexes are correct.
3. **Double-ingest race:** both devices ingest the same raw item. Exactly one op survives.
4. **Human + AI conflict:** the user edits a paragraph in Obsidian on the desktop while the phone
   ingests into the same page. No text is lost, and a conflict callout plus a Review item appear if
   needed.
5. **Undo:** undo a middle op after later ops touched the same pages. The compensating op removes
   only that source's contributions.
6. **Vault isolation:** a fiction capture ("Sara is diagnosed with diabetes" in story X) never
   creates a health claim about the user.
7. **Persian search:** queries with ي vs ی, with and without ZWNJ, Arabic digits, and mixed fa/en all
   find the expected pages.
8. **Bidi rendering:** golden tests for mixed paragraphs, lists, tables, and code inside RTL pages.
9. **Validator:** a malicious or buggy model output (writing to raw/, broken links, uncited claims,
   stale hash) is rejected and repaired or aborted correctly.
10. **MCP OAuth:** a full PKCE + DCR flow against a local test authorization server. Token refresh
    works, and tool approval policies are respected.
11. **Voice:** a simulated audio stream with barge-in stops TTS within 200 ms and captures the new
    utterance.

---

## 14. Milestones

Each milestone ends with a working build and updated `PROGRESS.md`.

**M0 — Foundations**

- Monorepo, CI for all 5 platforms, Flutter shell, flutter_rust_bridge wiring, design tokens and
  core components, i18n (fa/en) with RTL, `daftar_cli` skeleton.
- *Accept:* the app launches on all platforms, switches language and theme live, and golden tests
  run in CI.

**M1 — Capture + Repo + Sync**

- Onboarding (clone or init with PAT or SSH key), raw capture (text, voice recording, photo), local
  job queue, the full sync algorithm (Section 5) excluding op replay.
- *Accept:* scenarios 1 (without AI), 3, and the raw-only part of 2 pass; captures made offline on
  the phone appear on the desktop after sync.

**M2 — Providers + Transcribe + Ingest**

- Provider adapters, roles and model settings with Test buttons, STT, vision describe, router, the
  agent loop, internal tools, the changeset validator, commits, ledger, log, index generation, op
  replay on conflict.
- *Accept:* scenarios 1, 2, 6, and 9 pass. A spoken Persian journal note produces a correct journal
  day section, people links, and a proposed health claim.

**M3 — Wiki reading and editing + Search**

- The renderer with all syntaxes, backlinks, local graph, the editor, Persian-aware FTS, and the
  Wiki tab and desktop panes.
- *Accept:* scenarios 7 and 8 pass; the performance targets in Section 12 are met on the fixture
  repo.

**M4 — Audit and Review**

- The Activity screen with diffs, undo, compensating ops, move to vault, re-run with note, exclude,
  and the Review card stack.
- *Accept:* scenarios 4 and 5 pass; every op is reversible from the UI.

**M5 — Ask**

- Chat with streaming, citations, scopes, image input, "Save to wiki", and story co-writer mode.
- *Accept:* answers cite correctly on the fixture repo; no fabricated personal facts on questions
  whose answers are absent.

**M6 — Voice mode**

- VAD, streaming STT → LLM → TTS, barge-in, per-language voices, background audio, and transcript
  ingestion.
- *Accept:* scenario 11 passes; the latency target is met with a fast provider; the mode works
  hands-free for a 10-minute conversation.

**M7 — MCP**

- All transports and auth methods, per-device auth cards, tool policies, and resources/prompts.
- *Accept:* scenario 10 passes; tested against at least one real remote OAuth MCP server and one
  stdio server.

**M8 — Reflect and Lint**

- Scheduler, daily and weekly reviews, pattern proposals with evidence thresholds, lint
  (deterministic + verified LLM findings), notifications, and the wellbeing guardrail.
- *Accept:* a weekly review generated from the fixture month is accurate, cited, and useful; lint
  catches seeded contradictions and orphans with verified quotes.

**M9 — Polish and ship**

- Widgets, share extensions, quick actions, global hotkey, accessibility pass, performance pass, the
  full golden set, packaging and signing docs, a user guide in both languages
  (`docs/user-guide.fa.md`, `docs/user-guide.en.md`), and the SCHEMA.md template finalized.
- *Accept:* all scenario tests are green and release artifacts are built by CI for all platforms.

---

## 15. Prompts the app ships (write them carefully; keep them in `core/prompts/*.md`, versioned)

Write and iterate on these system prompts with the fixture repos and the mock and real providers:

- `router.md`
- `ingest.md`
- `query.md`
- `voice.md` (shorter sentences, spoken style, no Markdown, no lists, numbers spoken naturally,
  same-language replies)
- `vision_describe.md`
- `reflect_daily.md`
- `reflect_weekly.md`
- `lint.md`
- `compensate.md`
- `story_cowriter.md`

Each prompt:

- embeds the current `SCHEMA.md`, today's date and time zone, the vault list, and the user's
  preferred languages;
- states the claim rules, citation rules, and isolation rules;
- tells the model to prefer updating existing pages over creating near-duplicates (search first; the
  code also blocks duplicate slugs and aliases);
- tells the model to preserve the user's own words for emotional and personal content rather than
  paraphrasing them into clinical language.

Include a small **eval harness** (`daftar_cli eval`) that runs each prompt against fixture inputs
and checks structural expectations (pages touched, claims proposed, citations present, isolation
respected), so prompt changes can be regression-tested.

---

## 16. Definition of done

A person installs Daftar on an Android phone and a Linux laptop and connects the same empty private
GitHub repo on both. They then:

1. Configure one OpenAI-compatible provider for chat, STT, and TTS, and an Anthropic provider for
   ingest.
2. Record a week of mixed Persian and English voice notes, photos, and a few story ideas, mostly
   offline on the phone.

Afterwards:

- They open the laptop and see a clean, beautiful, interlinked wiki that also opens perfectly in
  Obsidian.
- They can ask by voice "این هفته بیشتر نگران چی بودم؟" ("What was I most worried about this week?")
  and get a spoken, cited answer.
- They can review and confirm a handful of claims.
- They can undo one mis-filed note in two taps.
- They have never seen a Git conflict.
