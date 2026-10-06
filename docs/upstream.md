# Upstream integration register

Praxilume uses the ArtCraft repositories as engineering references and, where compatible, as capability providers.

| Capability | Upstream repository | Praxilume boundary |
|---|---|---|
| Raster/image editing | storytold/photocraft | command adapter |
| Vector illustration | storytold/vectorcraft | command adapter |
| Video editing | storytold/filmcraft | timeline/media adapter |
| RAW/photo development | storytold/lightcraft | image pipeline adapter |
| PDF workbench | storytold/printcraft | document adapter |
| Motion/VFX | storytold/effectcraft | animation/render adapter |
| Page layout | storytold/designcraft | publishing adapter |

## Rule

Do not copy a subsystem merely because it exists upstream.

Prefer this sequence:

1. Reuse a stable upstream capability.
2. Wrap it with a small Praxilume adapter.
3. Normalize inputs and outputs into the Praxilume project/command model.
4. Preserve upstream license and attribution requirements.
5. Replace the adapter only when we have a measurable reason.

## License boundary

The upstream repositories expose Apache-2.0 and MIT license files plus repository notices. Any future vendored code must preserve the applicable upstream license and notice files and must be isolated under an explicit upstream or adapter boundary.

Praxilume's own source is Apache-2.0. This does not relicense upstream software.
