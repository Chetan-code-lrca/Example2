# Contributing

## Development

Run cargo fmt and cargo test for the workspace before opening a pull request.

Keep capability adapters isolated from the shared project model. Prefer small, reviewable changes.

New commands should include protocol/schema changes and tests for serialization and authorization behavior.

When reusing source from an external engine, record the upstream repository, revision, license, and notice requirements in docs/upstream.md before vendoring or redistributing it.
