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

**Milestone 0 — Foundation is implemented.**

The Rust workspace now contains:

```text
praxilume/
├── crates/
│   ├── core/             # shared project model + commands + policy
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
praxilume artcraft 7654 --token "$ARTCRAFT_TOKEN" --command app.status
```

## Architecture

```text
Praxilume UI / CLI / Agent / Automation
                  │
           praxilume.command/v1
                  │
          Praxilume runtime
                  │
      ┌───────────┼────────────┐
      │           │            │
  project      asset graph   policy
      │
 capability adapters
      │
 ArtCraft / other specialist engines
```

The core deliberately stays independent of renderers and codecs. Those belong behind capability boundaries.

## Verification policy

Every milestone follows:

```text
README updated
    ↓
implementation
    ↓
static verification
    ↓
GitHub Actions cargo fmt + cargo test
    ↓
promote verified commit to main
```

This environment does not contain a Rust toolchain, so local `cargo test` cannot be executed here. The repository CI workflow is configured to run formatting and workspace tests on GitHub.

## Repository naming

The connected GitHub API can create branches and write commits but does not expose repository-create or repository-rename operations. The codebase therefore uses the intended product name **Praxilume**, while the current remote bootstrap repository is still named `Example2`.

After the initial foundation is promoted, rename the repository to **`praxilume`** in GitHub Settings when the repository UI is available.

## Upstream reference

The seven ArtCraft applications we use as architectural and integration references are documented in [`docs/upstream.md`](docs/upstream.md).

Upstream code remains under its original licenses and notices. Praxilume does not relicense upstream code.

## License

Praxilume source is Apache-2.0.
