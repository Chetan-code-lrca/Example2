# Architecture

Praxilume separates the project model from specialist media implementations.

    UI / CLI / AI Agents
             |
       praxilume.command/v1
             |
       Policy + Command Bus
             |
      +------+------+------+
      |      |      |      |
    State  Assets Audit  Runtime
      |      |      |      |
      +------+------+------+
             |
      Capability Packs
             |
    ArtCraft / native / external

The shared core knows what a project is. Capability packs know how to manipulate media. This prevents the project format from becoming an accidental copy of one editor's internal state.

## Non-negotiable properties

- Structured, editable state.
- Explicit capability authorization.
- Content-addressed asset identity.
- No hidden network requirement for local workflows.
- Upstream license separation.
- Adapters that can be tested independently from the UI.
