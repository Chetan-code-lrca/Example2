# Security policy

Praxilume is an automation and agent control plane, so security is part of the core architecture.

## Security goals

- Agent actions are capability-gated.
- The default agent policy denies write and execute access.
- Local project roots are explicitly scoped.
- Upstream control tokens are never stored in project data.
- Network access is a distinct capability.
- Export operations are auditable.

## Reporting

Please report security issues privately through the repository's security advisory mechanism when it is enabled. Do not publish a reproducible exploit before a fix is available.
