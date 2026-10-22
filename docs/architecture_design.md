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
   - Mount order (decided 2026-09-18): engine defaults, then the agent profile, then the working directory, then `--dir` / `--ro-dir` in command-line order. A later mount covers an earlier one at the same path, so any explicitly allowed path overrides the default for that path with no special cases: `--dir /tmp` shows the host's `/tmp`, `--ro-dir .` makes the working directory read-only. cave warns on `--dir /` and `--dir ~`, which undo the boundary.
   - Path classification: concrete paths are defined in the embedded configuration data [`src/profiles.toml`](../src/profiles.toml). The classes and rationale are:
      - **Host runtime (RO)**: OS paths (`/usr`, `/etc`, `/bin`, `/sbin`, `/lib*`, `/opt`, `/var`, `/sys`) and installed toolchain binaries (`~/.local/bin`, `~/.cargo/bin`, `~/.rustup`, `~/.gitconfig`).
      - **Shared tool caches (RW)**: disposable package and build caches (`~/.cache`, `~/.cargo/registry`, `~/.cargo/git`, `~/.npm`) to avoid repeated downloads.
      - **Private tmpfs**: `$HOME`, `/tmp`, `/run`, `/var/tmp`.
      - **Hidden**: sensitive host directories (`~/.config`, `~/.ssh`, `~/.gnupg`, `~/.docker`, `/run/docker.sock`, `/run/user/$UID/bus`).
      - **Agent profile**: declared per-agent; Claude Code includes state (`~/.claude/`, `~/.claude.json`) as RW and runtime (`~/.local/share/claude`) as RO.

   - Machine-specific toolchain paths (e.g. a custom npm prefix such as `~/.npm_global`) are not defaults. Pass them with `--ro-dir`, or with a shell alias (`alias cave='cave --ro-dir ~/.npm_global'`) until the config-file candidate lands.

   - DNS: `/etc/resolv.conf` is a symlink into `/run` on systemd-resolved hosts, so the file it resolves to is bound read-only over it, keeping `/run` private. `--doctor` verifies resolution by resolving a name inside the sandbox.
   - `/var` is bound read-only on the assumption that `/var/run` is a symlink to `/run`. Where a distro still makes it a real directory, the Docker socket returns through that bind; `--doctor` checks this.
   - Environment (decided 2026-09-18): the host environment passes through, minus variables that point at hidden sockets (`SSH_AUTH_SOCK`, `DBUS_SESSION_BUS_ADDRESS`); `XDG_RUNTIME_DIR` points at a private `/run/user/$UID` (`0700`). An allowlist would never cover every toolchain variable (`JAVA_HOME`, `CARGO_HOME`, `NVM_DIR`, proxies), and environment secrets are not files, so they are outside the clumsy-agent scope.
   - Network: outbound network enabled for API calls and package downloads.
   - Agent profiles (decided 2026-09-18): each agent's state paths (RW), runtime paths and `~/.config` subpaths (RO), declared as data and interpreted by one engine. Built-in profiles are compiled into the binary and selected by the agent command's basename (`~/.local/bin/claude` → `claude`); an unknown command gets the engine defaults only, with a warning. Adding an agent is a data change; the format is backend-independent, so a future platform adds a backend, not new profiles.
   - Inheritance model (decided 2026-10-22): `$HOME` is an unconditional private tmpfs — no profile entry turns it off, and the only way to see the host home is the already-warned `--dir ~`. Host state is inherited by declaration: every path in `ro_home`, `rw_cache`, `state`, or `runtime` is bound from its own host path on top of that tmpfs, live and in place — same inode, no copy-in at start, no copy-out at exit. That is what lets `claude -c` inside cave resume a host session, and a session started inside cave appear on the host. It follows that a `state` entry grants live read-write access to host state, so an agent can corrupt or delete it (damage inside allowed directories is out of scope; snapshotting remains the candidate). `state` is therefore a privilege list to keep minimal, and read-only needs belong in `runtime`.
   - Whole-home inheritance (rejected 2026-10-22): binding the entire home read-only is the convenient alternative and would still block `rm -rf ~`, but a read is file access. The realistic clumsy-agent failure is `grep -r TOKEN ~` or a stray `cat ~/.aws/credentials` reaching the transcript, which is also where it leaves the machine. Declaring the few paths an agent needs costs one line per agent and keeps the Scope boundary intact.
   - Unknown agents (decided 2026-10-22): when no profile matches the basename, cave warns on stderr — naming the basename it tried, stating that `$HOME` is ephemeral so agent state will not persist, and pointing at `--dir` as the immediate escape hatch — then runs with engine defaults. Discarding an agent's state at exit in silence is exactly the quiet failure the fail-loudly rule exists to prevent: the agent re-onboards every session with no indication why. The warning belongs in `main.rs`; `build_mount_plan` stays pure.
   - Inherited-by-default judgment calls (decided 2026-10-22): `~/.npmrc` stays out of `ro_home`, so installing from a private registry fails inside cave until the path is added. It can hold `_authToken`, and inheriting it would hand every agent npm credentials for the convenience of one workflow; the user config file is the place to opt in per machine.
   - Configuration layering (decided 2026-10-22): the built-in table is compiled into the binary (`include_str!`) and is a complete policy on its own; a user file at `~/.config/cave/profiles.toml` layers over it.
      - **Append, not replace.** External lists extend the embedded ones. Whole-file replacement would force restating every default to add one path, and that copy would go stale as cave's defaults change.
      - **Downgrade by re-declaration.** External entries mount after the defaults and a later mount wins, so re-declaring an embedded path in a narrower class changes it: `~/.cache` listed under `ro_home` becomes read-only.
      - **Removal is not expressible**, by choice. Should a concrete need appear, it is one more class in the same table (`hide = [...]`, rendered as a tmpfs — the mechanism that already hides `$HOME`), not a new mechanism. Candidate, not built.
      - **User-global only.** A per-project `./.cave/profiles.toml` would sit in the read-write working directory, so an agent could widen its own boundary on the next run — accidental under this threat model, but a sandbox whose rules live inside the sandbox is the wrong shape. `~/.config/cave/` is hidden inside the sandbox, so the confining policy is unreadable from within. A per-project file would need an explicit opt-in flag.
      - **Parse failures are fatal and located.** A malformed user file exits non-zero naming file and line; ignoring it would run the default boundary while the user believed it was widened. The embedded table keeps `expect` — a build-time invariant covered by `test_load_config`.
      - `--doctor` prints the loaded sources and the effective merged table, since “why isn't my path mounted” is otherwise unanswerable without reading the source.

## Spike Outcomes & Decisions

Decided 2026-09-18 at Phase 1 Decision Check Point:
- **Isolation Engine**: Bubblewrap (`bwrap`) selected. Measured startup overhead of 9.28 ms (well within < 100ms threshold), zero root/daemon requirements, cleanly hides `/tmp` and `$HOME` with tmpfs.
- **Terminal Model**: Direct Terminal Inheritance selected. `bwrap` inherits controlling terminal descriptors directly (`isatty: True`); resize (`SIGWINCH`), Ctrl-C (`SIGINT`), and colors pass natively without an intermediary PTY relay.
- **Host /tmp Reuse**: Reusing host `/tmp` is supported strictly via the explicit `--dir /tmp` (or `--ro-dir /tmp`) option, which replaces the default private tmpfs. Generalized 2026-09-18 into the mount-order rule: every explicitly allowed path overrides the default for that path.
- **Implementation Language**: Rust selected. Near-zero process startup (< 2ms overhead), single static binary distribution with no runtime dependencies, type-safe CLI and profile data models, and direct kernel process replacement via `std::os::unix::process::CommandExt::exec`. Bash reconsidered 2026-09-18 and rejected: it matches Rust's startup, single-file distribution, and `exec`, and is easier to audit, but cave is primarily for the owner's own use, so auditability and distribution for other users weigh little; Rust kept by the owner's choice.
- **Backend Coupling**: `cave` orchestrates `bwrap` as a loosely-coupled external backend rather than re-implementing or embedding namespace syscalls in Rust. Preserves security auditing, avoids complex kernel mount edge cases, and retains future backend flexibility.

## Delivery Approach

Decided 2026-09-17.

- **Spike before choosing.** Backend, language, and terminal approach are chosen from measurements of a throwaway spike, not a paper comparison.
- **Walking skeleton.** Terminal handling and the file boundary ship together as one thin `cave claude`: without the boundary, terminal passthrough is plain `exec`, and the terminal approach depends on the backend.
- **Boundary tests first.** The Scope boundary is written as executable tests before the skeleton is built; passing them is its exit criterion.
- **Profiles instead of presets.** Per-agent support is profile data on the Phase 2 engine rather than a separate presets phase.
