# Roadmap

## Phase 1 — foundation

- Unified project and asset model
- Command envelope and policy primitives
- Content-addressed local asset storage
- ArtCraft control bridge
- CI, schema, and documentation

## Phase 2 — first real capability

Build an image capability adapter around an upstream raster engine rather than a second raster implementation.

- [x] Canonical command catalog and typed constructors for invert, crop, resize, brightness/contrast, layer creation and layer movement
- [x] PhotoCraft command-ID mapping through the authenticated control bridge
- [ ] Read-root-scoped image import/open using PhotoCraft's `doc.open` method
- [ ] Broader color adjustments and non-destructive filter stacks
- [ ] Write-root-scoped export using PhotoCraft's `doc.save` or render/export methods

Every operation must be represented as a Praxilume command and remain inspectable by an agent.

## Phase 3 — creative capability graph

- Vector capability pack
- Timeline/media capability pack
- Photo/RAW capability pack
- PDF/publishing capability pack
- Motion/VFX capability pack

All capabilities share the same project and asset graph.

## Phase 4 — AI-native authoring

- Plan -> approve -> execute agent loop
- JSON-Schema tool registry
- Vision-to-structured-object operations
- Local model providers
- Generation provenance and model metadata
- Human approval gates for destructive, export, and network actions

## Phase 5 — creative version control

- Branching projects
- Object-level diff
- Asset-aware merge
- Reversible operations
- Reproducible export manifests
