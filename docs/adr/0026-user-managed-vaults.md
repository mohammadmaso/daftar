# ADR-0026: User-managed vaults

* Status: accepted
* Date: 2026-09-26

## Context

The wiki's categories (vaults) were already stored in `.daftar/config.json`, but only as the five
defaults written at init. Nothing could add, rename, archive or remove a vault. The assistant also
learned about vaults in inconsistent ways:

* the router prompt hardcoded rules for `life`, `health`, `mind` and `work`;
* SCHEMA.md, which is written once and fed to every agent, has a fixed vault table;
* daily reflect, the story co-writer and undo got no vault list at all;
* the list was formatted in five different places.

## Decisions

**The config is the list; the app edits it.** `daftar_core::vaults` offers add, edit, archive,
restore and remove. The bridge exposes them as `LibraryHandle::{vault_settings, add_vault,
edit_vault, set_vault_archived, remove_vault}`, the CLI as `daftar vault …`, and the app as
Settings › Vaults. A vault's id (its folder) is a slug of the English name. It is unique among all
vaults, archived ones included, and it never changes: a rename only changes the titles, so links
into the vault stay valid.

**Remove only what is empty; archive the rest.** Removing a vault that still holds pages would
delete user knowledge, and the repo is the truth. So `remove` is refused while the folder has any
file besides the generated `index.md`, and the app offers Remove only for an empty vault. Archiving
hides a vault from the app, the router and every prompt, and blocks new AI writes into it. Undo
(compensating ops) may still write there. Its pages stay in the repo and it can be restored.

**Life and Stories are built in.** The code gives two vaults a role: `life` holds the journal and
weekly reviews and is the routing fallback, and `stories` is the isolated fiction vault. They can be
renamed and re-described, but not archived or removed (`config::is_builtin_vault`). Every other
vault, the defaults included, is the user's to change. Letting fiction live in any vault flagged
`fiction` is possible later. It would mean replacing the `stories` paths in tools, validate, ask,
lint and the prompts.

**One vault block for every agent.** `Config::vaults_for_prompt()` renders the active vaults as
`` - `id` (English · Persian): purpose ``, with fiction marked. `prompts::schema(lib)` appends that
list to SCHEMA.md under "Current vaults (authoritative)". The heading tells the agent to prefer this
list over the static table and to file into a new vault by its purpose. All agents that read the
schema use it: ingest, undo, ask, voice, story co-writer, lint, and daily and weekly reflect. The
router and query prompts also keep their own `{{vaults}}` list.

**Route by purpose (router v2).** The router's rules no longer name `health`, `mind` or `work`.
Instead it routes by each vault's purpose, which is the user's own description. The hints the old
rules carried (sleep, doctors, emotional states…) moved into the default purposes. The journal
fallback and the fiction rule stay.

**Vault changes are one settings commit.** A vault's `index.md` is created, retitled or removed
together with the config. `commit_local` commits both in the `settings:` commit, not as an external
edit.

## Consequences

* Existing libraries keep their SCHEMA.md. The appended block makes it correct without a migration.
* A vault added on one device appears on others after sync. The per-id merge of `vaults[]`
  (ADR-0016) handles concurrent edits.
* Claim zones (`health`, `mind`, `life/profile.md`) stay tied to those folders. A new vault holds
  plain pages, not claims.
