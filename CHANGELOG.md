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

Foundation CI passed rustfmt and the workspace test suite. The PhotoCraft bridge now includes unit tests for authentication framing, command framing, parameter validation, and upstream error reporting; this change is awaiting CI verification.
