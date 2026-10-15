# Praxilume

**A structured creative engine and AI control plane for local-first creative software.**

Praxilume is not a renamed ArtCraft application. It is the coordination layer around specialist creative engines: one structured project model, one command protocol, one asset/provenance graph, one permission boundary, and capability adapters.

## Design stance

ArtCraft is the reference implementation we learn from—not code we blindly duplicate. Its applications demonstrate the value of engine-first commands, JSON control, CLI automation, and native Rust media tooling. Praxilume keeps those ideas while adding a unified project/runtime layer.

We reuse specialist engines through explicit adapters instead of rebuilding image codecs, RAW processing, video codecs, PDF internals, vector path operations, and other mature subsystems from zero.

## What this project adds

- **Structured editable projects** instead of flattened AI output.
- **SHA-256 asset identity and provenance** for source, license, attribution, and tags.
- **One command protocol**: `praxilume.command/v1`.
- **Deny-by-default agent capabilities** for project, asset, network, export, and upstream-tool access.
- **Upstream adapters** so existing specialist engines can be reused instead of rewritten.
- **Atomic project persistence** with a deterministic project layout.
- **Capability-pack roadmap** for image, vector, video, motion, photo, publishing, and PDF workflows.

## Current milestone

**Phase 2 image editing is verified; Phase 3 now has an initial typed VectorCraft capability and loopback adapter.**

```text
praxilume/
├── crates/
│   ├── core/             # shared project model + commands + policy
│   ├── image-capability/ # typed image operations + canonical command catalog
│   ├── runtime/          # project persistence + content-addressed assets
│   ├── artcraft-bridge/  # controlled JSON-lines upstream adapter
│   └── cli/              # functional command line interface
├── docs/
├── schemas/
└── .github/workflows/
```

The CLI supports:

```bash
praxilume init demo.prax --name "First Project"
praxilume inspect demo.prax
praxilume rename demo.prax --name "Poster Lab"
praxilume import demo.prax ./image.png --extension png
praxilume command demo.prax --command project.rename --params '{"name":"Poster Lab"}'
praxilume image-commands  # list canonical image command IDs and upstream mappings
# Start PhotoCraft with dedicated roots; create exports/ beneath the write root first.
photocraft --control 7878 --control-token-file /private/path/photocraft-control.token \
  --automation-read-root /work/project --automation-write-root /work/project
export PHOTOCRAFT_CONTROL_TOKEN="$(cat /private/path/photocraft-control.token)"
praxilume photocraft-open 7878 assets/source.png
praxilume artcraft 7878 --command image.invert
praxilume artcraft 7878 --command image.crop --params '{"x":0,"y":0,"width":800,"height":600}'
praxilume photocraft-save 7878 exports/edited.png

# Start VectorCraft with roots that already exist; its control port has no authentication.
vectorcraft --control 7979 --automation-read-root /work/vector --automation-write-root /work/vector/out
praxilume vectorcraft-open 7979 assets/logo.svg
praxilume vectorcraft 7979 --command vector.rectangle --params '{"x":20,"y":20,"width":180,"height":80,"radius":8}'
praxilume vectorcraft 7979 --command vector.paint.fill --params '{"color":"#cc66aa"}'
praxilume vectorcraft-export 7979 out/logo.svg
praxilume vector-commands
```

## Architecture

```text
Praxilume UI / CLI / Agent / Automation
                  |
           praxilume.command/v1
                  |
          Praxilume runtime
                  |
      +-----------+-------------+
      |           |             |
  project      asset graph    policy
      |
 capability adapters
      |
 ArtCraft / other specialist engines
```

The core deliberately stays independent of renderers and codecs. Those belong behind capability boundaries.

### PhotoCraft bridge status

The first upstream adapter follows PhotoCraft's authenticated loopback protocol: authenticate first, then invoke `engine.execute`. It validates parameters and response IDs, surfaces upstream errors, and allows up to 65 seconds for PhotoCraft's documented 60-second response window. The CLI accepts `PHOTOCRAFT_CONTROL_TOKEN` when `--token` is omitted; prefer the environment variable to avoid exposing the secret in process arguments. The `praxilume.image/v1` catalog maps canonical commands for image edits, Hue/Saturation, Levels and Curves adjustment layers, and Smart Filter preparation, blur, parameter editing, visibility, ordering, deletion and enable/disable to verified PhotoCraft engine commands. The `photocraft-open` and `photocraft-save` commands use PhotoCraft's `app.open`/`app.save` methods. They reject absolute paths, parent traversal, backslashes and drive-qualified paths; PhotoCraft separately confines paths beneath its configured read/write roots. Export directories must already exist. To use non-destructive effects, create an adjustment layer or prepare the target layer for Smart Filters before adding a filter. For example: `praxilume artcraft 7878 --command image.smart_filter.prepare`, followed by `praxilume artcraft 7878 --command image.smart_filter.gaussian_blur --params '{"radius":3}'`. A regular raster layer may apply a filter destructively unless converted first.

### VectorCraft bridge status

VectorCraft uses a different desktop JSON-lines control protocol from PhotoCraft. Its socket has **no authentication**, so Praxilume hard-codes this adapter to `127.0.0.1`; never expose VectorCraft's control port to another host. Start VectorCraft with `--automation-read-root` and `--automation-write-root` so upstream file access is confined, then use `vectorcraft-open` and `vectorcraft-export` for root-scoped document I/O. Export parent directories must already exist.

The initial `praxilume.vector/v1` catalog provides typed rectangle, ellipse and SVG-path creation plus fill/stroke color operations. These commands act on an already-open document (except open/export), and the new shapes use the current VectorCraft drawing appearance. The adapter currently supports the five listed operations; grouping, boolean geometry, typography and multi-artboard workflow remain future work.

## Verification policy

Every milestone follows:

```text
README updated
    |
implementation
    |
static verification
    |
GitHub Actions: cargo fmt + cargo test
    |
promote verified commit to main
```

The image command pack passed [GitHub Actions run 38720441726](https://github.com/Chetan-code-lrca/Example2/actions/runs/38720441726) with 16 unit tests. Root-scoped PhotoCraft open/export passed [GitHub Actions run 38721088861](https://github.com/Chetan-code-lrca/Example2/actions/runs/38721088861) with 20 unit tests. Adjustment-layer and Smart Filter controls passed [GitHub Actions run 38721808097](https://github.com/Chetan-code-lrca/Example2/actions/runs/38721808097): formatting and workspace tests passed, with 24 unit tests plus doc-test stages. The new VectorCraft capability is awaiting CI verification.

## Repository naming

The connected GitHub API can create branches and write commits but does not expose repository-create or repository-rename operations. The codebase therefore uses the intended product name **Praxilume**, while the current remote bootstrap repository is still named `Example2`.

The intended final remote name is **`praxilume`**. Rename the repository in GitHub Settings when repository administration is available.

## Upstream reference

The seven ArtCraft applications are treated as capability references/providers:

- PhotoCraft
- VectorCraft
- FilmCraft
- LightCraft
- PrintCraft
- EffectCraft
- DesignCraft

Their repositories, integration boundaries, and license separation are documented in [`docs/upstream.md`](docs/upstream.md).

Praxilume does not relicense upstream software.

## License

Praxilume source is Apache-2.0.
