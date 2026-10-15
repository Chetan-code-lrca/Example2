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
praxilume artcraft 7878 --command image.adjustments.invert
```

Prefer the `PHOTOCRAFT_CONTROL_TOKEN` environment variable over `--token` so the credential is not included in the Praxilume process argument list. Keep the control endpoint bound to loopback and treat the token as a secret. Do not expose this port to untrusted networks.
