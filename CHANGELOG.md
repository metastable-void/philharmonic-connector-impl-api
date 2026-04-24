# Changelog

All notable changes to this crate are documented in this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-04-24

Initial substantive release: the trait-only API contract
between the `philharmonic-connector-service` framework and the
per-implementation `philharmonic-connector-impl-*` crates.

### Added

- `Implementation` async trait
  (`fn name(&self) -> &str` +
  `async fn execute(...) -> Result<JsonValue, ImplementationError>`),
  declared with `#[async_trait]` for dyn-compatibility and a
  `Send`-bounded returned future. See [the workspace doc on
  why `async_trait`][async-trait-rationale].
- Re-exports so impl crates can depend on this crate alone:
  - `ConnectorCallContext` and `ImplementationError` from
    `philharmonic-connector-common`.
  - `JsonValue` alias for `serde_json::Value`.
  - `async_trait` attribute macro from the `async-trait`
    crate.

### Scope boundaries

- No cryptography, no COSE handling, no network transport.
  Those live in the companion crates.
- No HTTP client dependency. Per the workspace rule, impl
  crates pick their own HTTP client — `reqwest` + `rustls-tls`
  for runtime code, per CONTRIBUTING.md §10.9.
- No tokio runtime dependency in the trait itself. Concrete
  impls use tokio; the trait is runtime-agnostic beyond the
  boxed-future shape that `#[async_trait]` implies.

[async-trait-rationale]: https://github.com/metastable-void/philharmonic-workspace/blob/main/docs/design/08-connector-architecture.md
