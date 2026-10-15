# Changelog

## Unreleased

### Added

- Initial Praxilume workspace.
- Structured project and asset model.
- Versioned command envelope and actor model.
- Capability policy primitives.
- Content-addressed local asset storage.
- PhotoCraft-authenticated JSON-lines control bridge with request/response validation.
- CI, protocol schema, security policy, roadmap, and upstream reuse documentation.

### Verification

Foundation CI passed rustfmt and the workspace test suite. The PhotoCraft bridge hardening is also verified by [GitHub Actions run 38719515469](https://github.com/Chetan-code-lrca/Example2/actions/runs/38719515469): `cargo fmt --all -- --check` and `cargo test --workspace` passed, including 10 unit tests across core, runtime, and bridge crates plus documentation-test stages.
