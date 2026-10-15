# Changelog

## Unreleased

### Added

- Initial Praxilume workspace.
- Structured project and asset model.
- Versioned command envelope and actor model.
- Capability policy primitives.
- Content-addressed local asset storage.
- PhotoCraft-authenticated JSON-lines control bridge with request/response validation.
- Typed image capability crate with versioned image edits, adjustment layers and Smart Filter operations.
- `praxilume image-commands` catalog discovery command.
- Hue/Saturation, Levels, and Curves adjustment-layer operations with parameter validation.
- Smart Filter preparation, Gaussian Blur, parameter editing, visibility, ordering, deletion, and enable/disable commands.
- Root-scoped PhotoCraft open/export commands using `app.open` and `app.save`.
- Initial `praxilume.vector/v1` capability pack for rectangle, ellipse, SVG path, fill and stroke.
- Separate unauthenticated-but-loopback-only VectorCraft bridge with root-scoped open/export.
- Portable path validation for absolute paths, traversal, and drive-qualified paths.
- CI, protocol schema, security policy, roadmap, and upstream reuse documentation.

### Verification

Foundation CI passed rustfmt and the workspace test suite. The PhotoCraft bridge hardening passed [GitHub Actions run 38719649555](https://github.com/Chetan-code-lrca/Example2/actions/runs/38719649555). The typed image command pack passed [GitHub Actions run 38720441726](https://github.com/Chetan-code-lrca/Example2/actions/runs/38720441726): Rustfmt and workspace tests passed, with 16 unit tests and doc-test stages successful. Root-scoped PhotoCraft open/export passed [GitHub Actions run 38721088861](https://github.com/Chetan-code-lrca/Example2/actions/runs/38721088861): Rustfmt and workspace tests passed, with 20 unit tests and doc-test stages successful. Adjustment-layer and Smart Filter controls passed [GitHub Actions run 38721808097](https://github.com/Chetan-code-lrca/Example2/actions/runs/38721808097): Rustfmt and workspace tests passed, with 24 unit tests and doc-test stages successful. Initial VectorCraft support passed [GitHub Actions run 38722780911](https://github.com/Chetan-code-lrca/Example2/actions/runs/38722780911): formatting and workspace tests passed, with 32 unit tests plus doc-test stages.
