# ADR-0021: MCP client on rmcp 3.4, with our own legacy SSE transport and HTTP stack

* Status: accepted
* Date: 2026-09-25

## Context
§10 asks for Streamable HTTP, legacy HTTP+SSE and desktop stdio transports; none/bearer/API key
(header or query)/custom header/OAuth 2.1 PKCE (RFC 9728, 8414, 7591, 8707, refresh)/client
credentials auth; per-device credentials; per-server tool policies. `rmcp` 3.4.1 (checked
2026-09-25, updated 2026-09-23) is the official SDK and uses reqwest 0.13 like us.

## Decision
* `rmcp` with `client`, `transport-streamable-http-client-reqwest`, `reqwest-tls-no-provider`,
  `auth`; `transport-child-process` only on non-mobile targets (stdio is desktop-only).
* rmcp no longer ships a legacy HTTP+SSE client; `mcp::LegacySse` implements rmcp's `Transport` for
  it (GET event stream, `endpoint` event, same-origin POSTs).
* Every HTTP request, including rmcp's OAuth state machine (via `OAuthHttpClient`), uses
  `tls::http_client` so the one trust store and proxy rules of ADR-0015 apply.
* Server configs live in `config.json` (`mcp`), secrets never: tokens, header values, stdio env
  values, client secrets and OAuth credentials come from the device's secure storage for each
  connection; refreshed OAuth credentials are handed back to be stored again.
* Tool names on the wire are `mcp__<server>__<tool>` (model APIs reject dots); the UI shows
  `mcp.<server>.<tool>`. Default policy is `auto_read_only`: tools annotated read-only run, others ask.
* HTTPS is required except for loopback addresses. Desktop OAuth uses a one-shot loopback listener
  on `127.0.0.1:<random>/callback`; mobile uses `daftar://oauth/callback` handed in by the app.

## Consequences
Scenario 10 runs against an in-test authorization + MCP server (DCR, PKCE verification, resource
parameter, refresh, policies, legacy SSE). The agent loop gained an async `ExternalTools` hook.
