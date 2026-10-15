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

## FilmCraft timeline capability

The `praxilume.timeline/v1` pack keeps timeline times as integer ticks and maps a small canonical operation set to FilmCraft commands: add edit, add edit on all tracks, razor and move clips. Its snapshot model normalizes sequence metadata, video/audio tracks, clip timing and source ranges, markers, and selection while leaving detailed effect/transition payloads available through the raw upstream response. The bridge validates typed parameters before sending a command; `--dry-run` prints the mapping without connecting to FilmCraft.

FilmCraft's control socket is unauthenticated. The adapter must remain loopback-only, and the upstream control port should only be enabled while in use.

## Non-negotiable properties

- Structured, editable state.
- Explicit capability authorization.
- Content-addressed asset identity.
- No hidden network requirement for local workflows.
- Upstream license separation.
- Adapters that can be tested independently from the UI.
