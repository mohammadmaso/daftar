# ADR-0015: One bundled trust store; rustls with ring

* Status: accepted
* Date: 2026-09-23

## Context
* reqwest 0.13's `rustls` feature pulls `aws-lc-rs` (cmake/NASM cross-builds for five platforms) and
  `rustls-platform-verifier`, which on Android needs JNI initialisation from Kotlin.
* libgit2 with vendored OpenSSL (Linux, Android) looks for CA certificates at compile-time paths that
  do not exist on those systems, so HTTPS remotes such as GitHub would fail on phones.

## Decision
* HTTP clients (AI providers, MCP) use rustls with the `ring` provider and the Mozilla roots from
  `webpki-root-certs`, via `reqwest::ClientBuilder::use_preconfigured_tls` (`tls::http_client`).
* libgit2 gets `set_ssl_cert_file`: the OS bundle when it is a plain file (Linux distributions),
  otherwise the same Mozilla roots written once as PEM into device-local state.

## Consequences
Identical trust behaviour everywhere, no JNI glue. Enterprise/self-signed CAs are honoured only
through the Linux system bundle; documented for self-hosted Gitea users (use a public CA).
