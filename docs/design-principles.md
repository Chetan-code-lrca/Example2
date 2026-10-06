# Design principles

### 1. Reuse specialization

Do not rebuild a mature media subsystem merely to own the whole stack. Integrate specialist engines behind capability boundaries.

### 2. Keep meaning editable

AI-assisted operations should preferably produce structured objects, layers, masks, vectors, captions, keyframes, or metadata rather than opaque flattened output.

### 3. Default deny for agents

Agents get only the capabilities they need. Project writes, exports, network access, and upstream process spawning require explicit authorization.

### 4. Provenance is data

Imported and generated assets retain identity, origin, license information, and reproducibility metadata.

### 5. One protocol, many frontends

CLI, desktop UI, integrations, and agents reach the same command model.

### 6. Local-first means operable offline

Core editing workflows do not depend on cloud round trips. Network-dependent features are explicit capabilities.
