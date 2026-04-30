# philharmonic-connector-impl-api

Trait-only API contract between the connector service framework
(`philharmonic-connector-service`) and per-implementation crates
(`philharmonic-connector-impl-*`).

## What this crate provides

The `Implementation` async trait that every connector
implementation adheres to:

```rust
#[async_trait]
pub trait Implementation: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(
        &self,
        config: &JsonValue,
        request: &JsonValue,
        ctx: &ConnectorCallContext,
    ) -> Result<JsonValue, ImplementationError>;
}
```

Also re-exports the caller-facing types the trait uses:
`ConnectorCallContext`, `ImplementationError`, `JsonValue`,
and the `async_trait` attribute macro.

## Design

This crate is the "trait seam" of the connector layer:
- Implementation crates depend on it to pick up the trait.
- The service framework depends on it to hold impls in
  `Box<dyn Implementation>`.

Both routes converge on the same symbol without either
depending on the other. No cryptography, no HTTP client,
no runtime dependency.

## License

Dual-licensed under `Apache-2.0 OR MPL-2.0`.

## Contributing

Developed as part of the
[Philharmonic workspace](https://github.com/metastable-void/philharmonic-workspace).
See
[`CONTRIBUTING.md`](https://github.com/metastable-void/philharmonic-workspace/blob/main/CONTRIBUTING.md).
