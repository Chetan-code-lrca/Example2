# Roadmap

## Phase 1 — foundation

- Unified project and asset model
- Command envelope and policy primitives
- Content-addressed local asset storage
- ArtCraft control bridge
- CI, schema, and documentation

## Phase 2 — first real capability

Build an image capability adapter around an upstream raster engine rather than a second raster implementation.

The first adapter should expose:

- image import/open
- crop and resize
- color adjustments
- layer creation and ordering
- non-destructive filter stacks
- export

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
