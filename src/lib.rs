//! Trait-only API contract between the
//! [`philharmonic-connector-service`] framework and per-implementation
//! connector crates (`philharmonic-connector-impl-*`).
//!
//! This crate is the "trait seam" of the connector layer. Per-
//! implementation crates depend on it to pick up the [`Implementation`]
//! trait they implement; the service framework depends on it to hold
//! impls in a `Box<dyn Implementation>` registry. Both routes converge
//! on the same symbol without either depending on the other.
//!
//! ## What lives here
//!
//! - The [`Implementation`] async trait that every connector
//!   implementation (`http_forward`, `llm_openai_compat`,
//!   `sql_postgres`, …) adheres to.
//! - Re-exports of the caller-facing types the trait signature uses:
//!   [`ConnectorCallContext`] and [`ImplementationError`] from
//!   [`philharmonic-connector-common`], [`JsonValue`] (an alias for
//!   [`serde_json::Value`]), and the [`async_trait`] attribute macro
//!   from the [`async-trait`] crate.
//!
//! Depending on this crate directly gives an impl crate every type it
//! needs to honour the trait — no transitive `philharmonic-connector-common`
//! dependency in the implementation's own `Cargo.toml` is required
//! (though it's harmless to add one if the implementation uses other
//! types from that crate).
//!
//! ## What deliberately does NOT live here
//!
//! - No cryptography. No key material. No COSE handling. No network
//!   transport. Those live in [`philharmonic-connector-client`] (the
//!   lowerer) and [`philharmonic-connector-service`] (the verifier +
//!   dispatcher).
//! - No HTTP client. Per the workspace rule in CONTRIBUTING.md §10.9,
//!   runtime HTTP clients in impl crates use [`reqwest`] (with
//!   `rustls-tls`) — this crate neither provides nor constrains that.
//! - No `tokio` runtime dependency. Concrete impls use tokio; the
//!   trait itself is runtime-agnostic (beyond requiring the
//!   [`async_trait`] boxed-future shape).
//!
//! ## Why `async_trait` and not native async-fn-in-traits
//!
//! See `docs/design/08-connector-architecture.md` §"Why `async_trait`
//! (in 2026)" in the workspace repo for the full rationale. Short
//! form: dyn-compatibility (the service framework holds
//! `Box<dyn Implementation>`) and `Send`-bound inference on the
//! returned future both still bite in 2026 when using native
//! async-fn-in-traits; the macro's per-call `Box` allocation is
//! negligible next to the external I/O every `execute` call performs.
//!
//! [`philharmonic-connector-service`]: https://crates.io/crates/philharmonic-connector-service
//! [`philharmonic-connector-client`]: https://crates.io/crates/philharmonic-connector-client
//! [`philharmonic-connector-common`]: https://crates.io/crates/philharmonic-connector-common
//! [`async-trait`]: https://crates.io/crates/async-trait
//! [`reqwest`]: https://crates.io/crates/reqwest

pub use async_trait::async_trait;
pub use philharmonic_connector_common::{ConnectorCallContext, ImplementationError};
pub use serde_json::Value as JsonValue;

/// One connector implementation's behavior: translate a decrypted
/// `config` + `request` pair into a response, or a typed
/// [`ImplementationError`].
///
/// Framework responsibilities (already done by the time `execute`
/// is called):
///
/// - COSE_Sign1 token signature verification.
/// - COSE_Encrypt0 payload decryption with the realm private key.
/// - Token claim validation (`exp`, `realm`, `payload_hash`).
/// - Payload `realm` / `impl` / `config` extraction.
/// - Lookup of the implementation in the binary's registry.
///
/// Implementation responsibilities (in `execute`):
///
/// - Deserialize `config` into the implementation's concrete
///   config type (fail with [`ImplementationError::InvalidConfig`]
///   on schema errors).
/// - Deserialize `request` into the implementation's concrete
///   request type (fail with [`ImplementationError::InvalidRequest`]).
/// - Perform the external I/O (HTTP call, SQL query, SMTP send, …).
/// - Produce a response [`JsonValue`] on success, or an
///   [`ImplementationError`] variant that categorizes the failure.
///
/// See `docs/design/08-connector-architecture.md` §"Security boundary"
/// and §"v1 implementation set" in the workspace repo for per-
/// capability wire protocols.
///
/// Implementations are `Send + Sync` so the service framework can
/// hold them behind `Box<dyn Implementation>` in a registry keyed by
/// [`Self::name`]. Registration is done at service startup; per-call
/// state lives on the stack of `execute` or in fields on `&self`.
#[async_trait]
pub trait Implementation: Send + Sync {
    /// The implementation's registration name, matching the `impl`
    /// field of the decrypted connector payload. Stable across the
    /// lifetime of the crate; changing it is a breaking change.
    ///
    /// Examples: `"http_forward"`, `"llm_openai_compat"`,
    /// `"sql_postgres"`. Lowercase snake_case by convention.
    fn name(&self) -> &str;

    /// Execute one connector call.
    ///
    /// - `config`: the decrypted `config` sub-object from the
    ///   connector payload. Shape is implementation-defined — each
    ///   implementation deserializes it into a concrete struct.
    /// - `request`: the script's cleartext request body. Passed
    ///   through from the lowerer unchanged.
    /// - `ctx`: verified token claims. Useful for logging, metrics,
    ///   and per-tenant behaviour. The framework has already
    ///   validated everything here; the implementation consumes it
    ///   as trusted metadata.
    ///
    /// Returns a [`JsonValue`] response on success, or a typed
    /// [`ImplementationError`] on any failure.
    async fn execute(
        &self,
        config: &JsonValue,
        request: &JsonValue,
        ctx: &ConnectorCallContext,
    ) -> Result<JsonValue, ImplementationError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use philharmonic_connector_common::{UnixMillis, Uuid};
    use serde_json::json;

    /// A trivial dummy impl used only to prove the trait's shape
    /// compiles as declared — it echoes `request` back. This is
    /// not part of the crate's public API; it lives under
    /// `#[cfg(test)]`.
    struct Echo;

    #[async_trait]
    impl Implementation for Echo {
        fn name(&self) -> &str {
            "echo"
        }

        async fn execute(
            &self,
            _config: &JsonValue,
            request: &JsonValue,
            _ctx: &ConnectorCallContext,
        ) -> Result<JsonValue, ImplementationError> {
            Ok(request.clone())
        }
    }

    /// Proves the trait is `Send + Sync` + dyn-compatible: a
    /// `Box<dyn Implementation>` actually constructs, and the
    /// service framework's intended usage pattern compiles.
    #[test]
    fn implementation_is_dyn_compatible() {
        let boxed: Box<dyn Implementation> = Box::new(Echo);
        assert_eq!(boxed.name(), "echo");
    }

    /// Verifies the `execute` signature compiles and runs
    /// end-to-end through the `Box<dyn Implementation>` shape
    /// the service framework will use. Tokio is a dev-dep
    /// only — the trait itself is runtime-agnostic.
    #[tokio::test]
    async fn execute_round_trips_request_on_dummy_impl() {
        let echo: Box<dyn Implementation> = Box::new(Echo);
        let ctx = ConnectorCallContext {
            tenant_id: Uuid::nil(),
            instance_id: Uuid::nil(),
            step_seq: 0,
            config_uuid: Uuid::nil(),
            issued_at: UnixMillis(0),
            expires_at: UnixMillis(1),
        };
        let config = json!({});
        let request = json!({"hello": "world"});

        let result = echo.execute(&config, &request, &ctx).await.unwrap();
        assert_eq!(result, json!({"hello": "world"}));
    }
}
