# ADR-0006: Appearance preferences are device-local

* Status: accepted
* Date: 2026-09-23

Theme, UI language and text size differ legitimately between a phone and a laptop, so they are stored
in platform preferences, not in the synced `.daftar/config.json`. Settings that must agree across
devices (vaults, providers without secrets, MCP servers, reflect schedule) go to config.json.
