# Changelog

## Unreleased

### Added

- Initial Praxilume workspace.
- Structured project and asset model.
- Versioned command envelope and actor model.
- Capability policy primitives.
- Content-addressed local asset storage.
- PhotoCraft-authenticated JSON-lines control bridge with request/response validation.
- Typed image capability crate with six canonical image-edit commands.
- `praxilume image-commands` catalog discovery command.
- CI, protocol schema, security policy, roadmap, and upstream reuse documentation.

### Verification

Foundation CI passed rustfmt and the workspace test suite. The PhotoCraft bridge hardening passed [GitHub Actions run 38719649555](https://github.com/Chetan-code-lrca/Example2/actions/runs/38719649555). The new typed image command pack is awaiting CI verification.
