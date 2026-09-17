# Architecture Design — cave

## System Overview

`cave` is a lightweight CLI wrapper and execution harness. When invoked with `cave <cmd> [args...]`, it sets up an isolated sandbox boundary, mounts necessary volumes, forwards environment variables/credentials securely, and attaches the user's interactive terminal directly to the sandboxed process.

```
+-------------------------------------------------------------+
| Host Machine                                                |
|                                                             |
|   User Terminal (PTY / stdin / stdout / stderr)             |
|          |                                                  |
|          v                                                  |
|      cave CLI (orchestrator & terminal passthrough)         |
|          |                                                  |
|   +------v-----------------------------------------------+  |
|   | Sandbox Boundary (bwrap / container / namespace)     |  |
|   |                                                      |  |
|   |  - CWD mounted (RW)                                  |  |
|   |  - Root filesystem (RO / ephemeral overlay)          |  |
|   |  - Sensitive host paths (~/.ssh, ~/.aws) masked      |  |
|   |  - Environment / API keys passed securely            |  |
|   |  - Process running: `<agent> [args...]`              |  |
|   +------------------------------------------------------+  |
+-------------------------------------------------------------+
```

## Architectural Components

1. **CLI & Process Dispatcher**:
   - Parses `cave` flags vs. forwarded agent commands.
   - Allocates/attaches pseudo-terminal (PTY) and manages terminal raw mode.
2. **Isolation Engine**:
   - Candidate backends:
     - **Bubblewrap (`bwrap`)**: Linux unprivileged user namespaces, sub-millisecond startup, zero daemon requirement.
     - **Container backend (Docker / Podman)**: Cross-platform portability where unprivileged namespaces are unavailable.
3. **Filesystem & Environment Policy**:
   - Project directory: bind-mounted read-write with matching host UID/GID.
   - Home directory: ephemeral tmpfs or filtered mount, selectively passing agent config (`~/.claude.json`, etc.) while masking credentials.
   - Network: outbound network enabled for API calls and package downloads.
