# ADR-0019: Ask verifies citations in code and enforces scopes in the tools

* Status: accepted
* Date: 2026-09-25

## Context
§4.3 requires answers that cite wiki pages and captures as tappable links, never invent personal
facts, and a story mode whose retrieval is restricted to one story.

## Decision
* Every wikilink in an answer is resolved (Obsidian rules) before the app sees it. Links that do not
  resolve are rewritten to their label as plain text and reported, so a tappable citation always
  opens a real page.
* Scopes (a vault, a story) are enforced by the read tools (`read_prefix`): `page_read`, `search`,
  `index_read` only see the scope; raw captures are closed in story mode. The prompt states the scope
  too, but correctness does not depend on the model.
* Ask runs read-only (`Scope::ReadOnly`, no `review_add`). "Save to wiki" turns the question and
  answer into a `chat-answer` capture filed by the normal ingest pipeline; story drafts are written as
  new pages under `stories/<story>/drafts/` (one `save-answer` op), never over chapters.
* The wellbeing card is shown when the model ends with `[[talk-to-someone]]` or when a small
  bilingual phrase list (`wellbeing::signals_crisis`) matches the question.

## Consequences
The model can still misread a page, but it cannot show a link to something that does not exist or
read outside the chosen scope. Answer quality on real providers is covered by `daftar eval`.
