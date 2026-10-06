# Praxilume

**A structured creative engine and AI control plane for local-first creative software.**

Praxilume is not a renamed ArtCraft application. It is the coordination layer around specialist creative engines: one structured project model, one command protocol, one asset/provenance graph, one permission boundary, and capability adapters.

## Design stance

ArtCraft is the reference implementation we learn from—not code we blindly duplicate. Its applications demonstrate the value of engine-first commands, JSON control, CLI automation, and native Rust media tooling. Praxilume keeps those ideas while adding a unified project/runtime layer.

We reuse specialist engines through explicit adapters instead of rebuilding image codecs, RAW processing, video codecs, PDF internals, vector path operations, and other mature subsystems from zero.

## What this project adds

- Structured editable project state instead of flattened AI output.
- SHA-256 content identity and asset provenance.
- A single `praxilume.command/v1` envelope for UI, CLI, automation, and agents.
- Agent capabilities with deny-by-default authorization.
- An upstream adapter boundary for compatible ArtCraft control channels.
- Deterministic project persistence with atomic JSON writes.
- A roadmap for image, vector, video, motion, photo, publishing, and PDF capability packs.

## Repository status

**Milestone 0 — Foundation**

The workspace is being bootstrapped on a temporary GitHub repository because the connected GitHub interface does not expose repository-creation or rename operations. The intended final repository name is **`praxilume`**.

Current work is developed on `praxilume-foundation` and promoted to `main` only after verification.

## Workspace

```text
praxilume/
├── crates/
│   ├── core/             # shared project + command + policy model
│   ├── runtime/          # project persistence + content-addressed assets
│   ├── artcraft-bridge/  # controlled JSON-lines upstream adapter
│   └── cli/              # praxilume command line interface
├── docs/
├── schemas/
└── .github/workflows/
```

## Quick start

```bash
cargo test --workspace
cargo run -p praxilume -- init demo.prax --name "First Project"
cargo run -p praxilume -- inspect demo.prax
```

## Integration principle

```text
Praxilume UI / CLI / Agent / Automation
                  │
            command envelope
                  │
          Praxilume runtime
                  │
      ┌───────────┼────────────┐
      │           │            │
  own project   asset graph  policy
      │
  capability adapters
      │
  ArtCraft / other specialist engines
```

Upstream attribution and licensing are documented in [`docs/upstream.md`](docs/upstream.md).

## License

Praxilume source is Apache-2.0. Upstream components retain their own licenses, notices, and attribution requirements.
