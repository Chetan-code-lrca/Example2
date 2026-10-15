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


## PhotoCraft control protocol

The current bridge implementation targets PhotoCraft's loopback JSON-lines control endpoint. Each new TCP connection must:

1. Send an `auth` frame containing the control token and require an `ok: true` response.
2. Send the Praxilume command as an `engine.execute` request with `command` and `params`.
3. Match the response ID to the request and surface upstream errors.

The adapter accepts object or null command parameters, keeps the token out of command frames, and uses a 65-second read timeout for PhotoCraft's documented 60-second response window. Other ArtCraft applications need their own verified protocol adapters; shared branding is not evidence of wire compatibility.

Example after launching PhotoCraft with `--control 7878` and opening a document:

```bash
export PHOTOCRAFT_CONTROL_TOKEN="$(cat /private/path/photocraft-control.token)"
praxilume image-commands
praxilume artcraft 7878 --command image.invert
praxilume artcraft 7878 --command image.crop --params '{"x":0,"y":0,"width":800,"height":600}'
```

Prefer the `PHOTOCRAFT_CONTROL_TOKEN` environment variable over `--token` so the credential is not included in the Praxilume process argument list. Keep the control endpoint bound to loopback and treat the token as a secret. Do not expose this port to untrusted networks.


## Initial image command pack

The current 16-command catalog is versioned as `praxilume.image/v1`. It exposes canonical IDs, typed request constructors, and a JSON catalog through `praxilume image-commands`.

| Praxilume ID | PhotoCraft engine command |
|---|---|
| `image.invert` | `image.adjustments.invert` |
| `image.crop` | `image.crop` |
| `image.resize` | `image.imageSize` |
| `image.brightness_contrast` | `image.adjustments.brightnessContrast` |
| `image.layer.create` | `layer.new.layer` |
| `image.layer.move` | `layer.moveTo` |

The typed API rejects non-positive crop/resize dimensions and self-target layer moves before dispatch. The bridge keeps backwards-compatible pass-through for native PhotoCraft command IDs. The six editing commands target an already-open document. The CLI now also exposes `praxilume photocraft-open <port> <relative-path>` and `praxilume photocraft-save <port> <relative-path>`, using PhotoCraft desktop control methods `app.open` and `app.save`. Both validate that the path is relative and normalized before dispatch. PhotoCraft then resolves the path beneath the configured `--automation-read-root` or `--automation-write-root`; start the app with distinct roots where practical. Output parent directories must already exist.


## Adjustment layers and Smart Filters

The current `praxilume.image/v1` catalog also exposes the following verified PhotoCraft mappings:

| Praxilume ID | PhotoCraft engine command | Notes |
|---|---|---|
| `image.adjustment_layer.hue_saturation` | `layer.newAdjustmentLayer.hueSaturation` | Non-destructive adjustment layer |
| `image.adjustment_layer.levels` | `layer.newAdjustmentLayer.levels` | Non-destructive adjustment layer |
| `image.adjustment_layer.curves` | `layer.newAdjustmentLayer.curves` | Non-destructive adjustment layer |
| `image.smart_filter.prepare` | `filter.convertForSmartFilters` | Convert the active/selected content first |
| `image.smart_filter.gaussian_blur` | `filter.blur.gaussianBlur` | Adds to the Smart Filter stack when prepared |
| `image.smart_filter.set_params` | `layer.smartFilter.setParams` | Merge a parameter object into the indexed filter |
| `image.smart_filter.set_visible` | `layer.smartFilter.setVisible` | Show/hide one stack entry |
| `image.smart_filter.move` | `layer.smartFilter.move` | Reorder one stack entry |
| `image.smart_filter.delete` | `layer.smartFilter.delete` | Remove one stack entry |
| `image.smart_filter.toggle` | `layer.smartFilter.disableSmartFilters` | `enabled: true/false` |

Typed constructors validate hue/saturation ranges, levels black/white/gamma inputs, monotonic curves input coordinates, Gaussian Blur radius and object-shaped Smart Filter parameters before dispatch. Optional layer IDs are included only when supplied.

Non-destructive behavior is capability-dependent: prepare a layer for Smart Filters before adding blur, and use adjustment-layer commands for Hue/Saturation, Levels or Curves. PhotoCraft's Smart Filter format support is not identical across every file format; this adapter does not promise that every filter is preserved by every export format.
