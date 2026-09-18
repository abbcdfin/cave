# Architecture Design — cave

## System Overview

`cave` is a lightweight CLI wrapper and execution harness. When invoked with `cave <cmd> [args...]`, it sets up an isolated sandbox boundary, mounts necessary volumes, forwards environment variables and agent credentials, and attaches the user's interactive terminal directly to the sandboxed process.

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
|   |  - Allowed directories mounted (RW)                  |  |
|   |  - Host runtime (RO)                                 |  |
|   |  - Rest of host hidden (home, /tmp, /run)            |  |
|   |  - Environment / API keys passed through             |  |
|   |  - Process running: `<agent> [args...]`              |  |
|   +------------------------------------------------------+  |
+-------------------------------------------------------------+
```

## Architectural Components

1. **CLI & Process Dispatcher**:
   - Parses `cave` options, which precede the agent command, and passes everything from the agent command on unchanged.
   - Terminal: the agent uses the host terminal directly, with no PTY relay (Phase 1 outcome).
2. **Isolation Engine**:
   - Selection criteria:
     - Enforces the Scope boundary by hiding, not just denying: private `/tmp` and `/run`, hidden home.
     - Adds < 100ms to startup.
     - Shows the host runtime at its host paths, so agents and toolchains run as installed.
     - Needs no root, setuid helper, or daemon.
     - Runs on the target distros, including those that restrict unprivileged user namespaces.
   - Candidate backends:
     - **Bubblewrap (`bwrap`)**: Linux unprivileged user namespaces, sub-millisecond startup, zero daemon requirement.
     - **Landlock**: no namespaces and near-zero startup, but denies access instead of hiding paths and cannot give a private `/tmp`.
     - **Container backend (Docker / Podman)**: Fallback where unprivileged user namespaces are unavailable or restricted.
   - Excluded: **systemd-nspawn**, which requires root.
   - Implementation language: chosen at the Phase 1 Decision Check Point by startup overhead, ease of distribution, and fit for the terminal approach (a PTY relay needs signal and PTY handling; direct use does not).
3. **Filesystem & Environment Policy**:
   - Boundary: defined in [product_specification.md › Scope](product_specification.md#scope).
   - Allowed directories: bind-mounted at their host paths with matching host UID/GID; read-write, or read-only for `--ro-dir`.
   - `/tmp` and `/run`: private ephemeral `tmpfs` by default. If `--dir /tmp` or `--ro-dir /tmp` is passed, the private tmpfs is skipped and the host's `/tmp` is bind-mounted directly.
   - Path classification (Phase 1 spike, host-verified):

     | Class | Paths |
     | :--- | :--- |
     | Agent state (RW) | `~/.claude/`, `~/.claude.json` — persist, so login and `claude -c` work |
     | Host runtime (RO) | `/usr`, `/etc`, `/bin`, `/sbin`, `/lib*`, `/opt`, `/var`, `/sys`; toolchains installed in home: `~/.local/bin`, `~/.local/share/claude`, `~/.cargo/bin`, `~/.rustup`, `~/.npm_global`, `~/.gitconfig` |
     | Private tmpfs | `$HOME`, `/tmp`, `/run`, `/var/tmp` |
     | Hidden | everything else, including `~/.ssh`, `~/.gnupg`, `~/.docker`, `/run/docker.sock`, `/run/user/$UID/bus` |
     | Open, Phase 2 decides | `~/.cache`, `~/.config` — hidden today, so every download repeats and config writes vanish silently |

   - DNS: `/etc/resolv.conf` is a symlink into `/run` on systemd-resolved hosts, so the file it resolves to is bound read-only over it, keeping `/run` private. `--doctor` verifies resolution by resolving a name inside the sandbox.
   - `/var` is bound read-only on the assumption that `/var/run` is a symlink to `/run`. Where a distro still makes it a real directory, the Docker socket returns through that bind; `--doctor` checks this.
   - Agent state is the per-agent part of this table and becomes profile data; the other classes are engine defaults.
   - Network: outbound network enabled for API calls and package downloads.
   - Agent profiles: each agent's state paths and environment, declared as data and interpreted by one engine. Adding an agent is a data change; the format is backend-independent, so a future platform adds a backend, not new profiles.

## Spike Outcomes & Decisions

Decided 2026-09-18 at Phase 1 Decision Check Point:
- **Isolation Engine**: Bubblewrap (`bwrap`) selected. Measured startup overhead of 9.28 ms (well within < 100ms threshold), zero root/daemon requirements, cleanly hides `/tmp` and `$HOME` with tmpfs.
- **Terminal Model**: Direct Terminal Inheritance selected. `bwrap` inherits controlling terminal descriptors directly (`isatty: True`); resize (`SIGWINCH`), Ctrl-C (`SIGINT`), and colors pass natively without an intermediary PTY relay.
- **Host /tmp Reuse**: Reusing host `/tmp` is supported strictly via the explicit `--dir /tmp` (or `--ro-dir /tmp`) option, which replaces the default private tmpfs.
- **Implementation Language**: Rust selected. Near-zero process startup (< 2ms overhead), single static binary distribution with no runtime dependencies, type-safe CLI and profile data models, and direct kernel process replacement via `std::os::unix::process::CommandExt::exec`.
- **Backend Coupling**: `cave` orchestrates `bwrap` as a loosely-coupled external backend rather than re-implementing or embedding namespace syscalls in Rust. Preserves security auditing, avoids complex kernel mount edge cases, and retains future backend flexibility.

## Delivery Approach

Decided 2026-09-17.

- **Spike before choosing.** Backend, language, and terminal approach are chosen from measurements of a throwaway spike, not a paper comparison.
- **Walking skeleton.** Terminal handling and the file boundary ship together as one thin `cave claude`: without the boundary, terminal passthrough is plain `exec`, and the terminal approach depends on the backend.
- **Boundary tests first.** The Scope boundary is written as executable tests before the skeleton is built; passing them is its exit criterion.
- **Profiles instead of presets.** Per-agent support is profile data on the Phase 2 engine rather than a separate presets phase.
