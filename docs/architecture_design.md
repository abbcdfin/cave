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
   - Mount order (decided 2026-09-18, refined 2026-10-23): engine defaults, then the agent profile (`state` RW mounts rendered before `runtime` RO mounts so read-only subpaths override parent read-write directories), then the working directory, then `--dir` / `--ro-dir` in command-line order. A later mount covers an earlier one at the same path, so any explicitly allowed path overrides the default for that path with no special cases: `--dir /tmp` shows the host's `/tmp`, `--ro-dir .` makes the working directory read-only. cave warns on `--dir /` and `--dir ~`, which undo the boundary.
   - Path classification: concrete paths are defined in the embedded configuration data [`src/profiles.toml`](../src/profiles.toml). The classes and rationale are:
      - **Host runtime (RO)**: OS paths (`/usr`, `/etc`, `/bin`, `/sbin`, `/lib*`, `/opt`, `/var`, `/sys`) and installed toolchain binaries (`~/.local/bin`, `~/.cargo/bin`, `~/.rustup`, `~/.gitconfig`, `~/.local/share/uv`).
      - **Shared tool caches (RW)**: disposable package and build caches (`~/.cache`, `~/.cargo/registry`, `~/.cargo/git`, `~/.npm`) to avoid repeated downloads.
      - **Private tmpfs**: `$HOME`, `/tmp`, `/run`, `/var/tmp`, `/etc/ssh/ssh_config.d` (see SSH below). A listed path missing on the host is skipped.
      - **Hidden**: sensitive host directories (`~/.config`, `~/.ssh` apart from `config` and `known_hosts`, `~/.gnupg`, `~/.docker`, `/run/docker.sock`, `/run/user/$UID/bus`).
      - **Agent profile**: declared per-agent; Claude Code includes state (`~/.claude/`, `~/.claude.json`) as RW and runtime (`~/.local/share/claude`) as RO.

   - Machine-specific toolchain paths (e.g. a custom npm prefix such as `~/.npm_global`) are not defaults. Pass them with `--ro-dir`, or with a shell alias (`alias cave='cave --ro-dir ~/.npm_global'`) until the config-file candidate lands.

   - DNS: `/etc/resolv.conf` is a symlink into `/run` on systemd-resolved hosts, so the file it resolves to is bound read-only over it, keeping `/run` private. (Sandbox DNS resolution check is verified in `--doctor`).
   - `/var` is bound read-only on the assumption that `/var/run` is a symlink to `/run`. Where a distro still makes it a real directory, the Docker socket returns through that bind. (Checked in `--doctor` to ensure `/var/run` is a symlink to `/run`).
   - Environment (decided 2026-09-18, refined 2026-10-23): the host environment passes through, minus variables that point at hidden sockets (`DBUS_SESSION_BUS_ADDRESS`, and `SSH_AUTH_SOCK` when no agent is forwarded; see SSH below); `XDG_RUNTIME_DIR` points at a private `/run/user/$UID` (`0700`). An allowlist would never cover every toolchain variable (`JAVA_HOME`, `CARGO_HOME`, `NVM_DIR`, proxies), and environment secrets are not files, so they are outside the clumsy-agent scope.
   - Network: outbound network enabled for API calls and package downloads.
   - SSH (decided 2026-10-23, Phase 5): the host ssh-agent is forwarded by default. The socket `$SSH_AUTH_SOCK` names is bound read-only to `/run/user/$UID/ssh-agent.sock` inside the private runtime dir and `SSH_AUTH_SOCK` points there; `~/.ssh/config` and `~/.ssh/known_hosts` are `ro_home` defaults; private keys stay hidden. `--no-ssh` turns it off for one session. `SSH_AUTH_SOCK` unset: nothing is forwarded, silently (`--doctor` reports it); set but the socket is missing: one stderr warning, then the session runs without it.
      - **Why forward at all.** The threat is a clumsy agent, and git over ssh is daily work; without it every push needs the user outside the sandbox.
      - **Why the agent, not the key files.** An agent that can read a key file will eventually print it while debugging a failed push, and its context leaves the machine. The ssh-agent signs without disclosing the key, so `~/.ssh` stays hidden as Scope promises. Unlike the Docker socket and the session bus, the ssh-agent reaches no host files.
      - **Accepted costs.** The agent can push and run remote commands with every key loaded into the ssh-agent; which keys are loaded is the user's control, and `ssh-add -c` makes the host confirm each signature. It can also run `ssh-add -D` and empty the host agent. Read-only `known_hosts` cannot record a new host, so a first connection to one fails non-interactively.
      - **System drop-ins are emptied** (decided 2026-10-23): inside the user namespace, root-owned host files appear owned by `nobody`, and OpenSSH refuses any `Include`d config file not owned by root or the user (`Bad owner or permissions on /etc/ssh/ssh_config.d/…`), so `ssh` failed before connecting. `/etc/ssh/ssh_config.d` is a private tmpfs; the main `/etc/ssh/ssh_config` is read without that check and stays. The cost is the drop-ins: on the dogfood host, systemd's and libvirt's VM proxies (which need host `/run` sockets the sandbox cannot reach anyway) and `ServerAliveInterval 120` (belongs in `~/.ssh/config`). Rendering user-owned copies was rejected: it needs file contents passed into bwrap at launch and still fails on nested includes (Fedora's crypto-policy file).
      - **Policy stays pure.** `main.rs` reads `SSH_AUTH_SOCK` and passes the socket path into `build_mount_plan`, as it does with the config.
   - Agent profiles (decided 2026-09-18, refined 2026-10-23): each agent's state paths (RW), runtime paths and `~/.config` subpaths (RO), declared as data and interpreted by one engine. Built-in profiles are compiled into the binary and selected by the agent command's basename (`~/.local/bin/claude` → `claude`); an unknown command gets the engine defaults only, with a warning. Standard shells (`sh`, `bash`, `zsh`) declare empty profiles (`state = []`, `runtime = []`) to silence unknown-agent warnings for shell workflows while preserving ephemeral `$HOME` semantics. Supported profiles:
      - **Claude Code (`claude`)**: `~/.claude/` (RW state), `~/.claude.json` (RW file state with `init = "{}"`), `~/.local/share/claude` (RO runtime).
      - **Antigravity CLI (`agy`)**: `~/.gemini` (RW state). `~/.gemini/antigravity-cli/bin` is part of mutable state (not read-only runtime) because `agy` dynamically writes its execution bridge `agentapi` there per session. GUI directories (`~/.antigravity`, `~/.config/Antigravity`) remain hidden.
      - **Aider (`aider`)**: `~/.aider` (RW state). Hand-written user config `~/.aider.conf.yml` is `runtime` (RO) to protect it from agent modification. Toolchain paths (`~/.local/share/uv`) are in `ro_home` defaults so uv tool symlinks resolve.
   - Inheritance model (decided 2026-10-22): `$HOME` is an unconditional private tmpfs — no profile entry turns it off, and the only way to see the host home is the already-warned `--dir ~`. Host state is inherited by declaration: every path in `ro_home`, `rw_cache`, `state`, or `runtime` is bound from its own host path on top of that tmpfs, live and in place — same inode, no copy-in at start, no copy-out at exit. That is what lets `claude -c` inside cave resume a host session, and a session started inside cave appear on the host. It follows that a `state` entry grants live read-write access to host state, so an agent can corrupt or delete it (damage inside allowed directories is out of scope; snapshotting remains the candidate). `state` is therefore a privilege list to keep minimal, and read-only needs belong in `runtime`.
   - Single-file bind mount and kernel rename constraint (decided 2026-10-22): when a file is bind-mounted directly (`~/.claude.json`), the target inside the mount namespace becomes an active mount point. In Linux VFS, an atomic file update via `rename(temp, target)` over an active mount point fails with `EBUSY`. Claude Code performs in-place writes (`open(..., O_WRONLY|O_TRUNC)`), which succeeds on the existing inode and propagates immediately to the host. However, creating a 0-byte file prior to mounting left invalid JSON on first run; profile entries declare explicit `init = "{}"` so fresh state files are valid JSON from inception. For tools that do require atomic replacement (`rename`), enclosing directory binds (rather than single-file binds) are necessary.
   - Whole-home inheritance (rejected 2026-10-22): binding the entire home read-only is the convenient alternative and would still block `rm -rf ~`, but a read is file access. The realistic clumsy-agent failure is `grep -r TOKEN ~` or a stray `cat ~/.aws/credentials` reaching the transcript, which is also where it leaves the machine. Declaring the few paths an agent needs costs one line per agent and keeps the Scope boundary intact.
   - Unknown agents (decided 2026-10-22): when no profile matches the basename, cave warns on stderr — naming the basename it tried, stating that `$HOME` is ephemeral so agent state will not persist, and pointing at `--dir` as the immediate escape hatch — then runs with engine defaults. Discarding an agent's state at exit in silence is exactly the quiet failure the fail-loudly rule exists to prevent: the agent re-onboards every session with no indication why. The warning belongs in `main.rs`; `build_mount_plan` stays pure.
   - Inherited-by-default judgment calls (decided 2026-10-22): `~/.npmrc` stays out of `ro_home`, so installing from a private registry fails inside cave until the path is added. It can hold `_authToken`, and inheriting it would hand every agent npm credentials for the convenience of one workflow; the user config file is the place to opt in per machine.
   - Configuration layering (decided 2026-10-22): the built-in table is compiled into the binary (`include_str!`) and is a complete policy on its own; a user file at `~/.config/cave/profiles.toml` layers over it.
      - **Append, not replace.** External lists extend the embedded ones. Whole-file replacement would force restating every default to add one path, and that copy would go stale as cave's defaults change.
      - **Re-declaration moves between classes.** In `merge_user_config`, re-declaring an embedded path in a different list moves that path (retained in the new list and removed from the prior list), permitting both narrowing (downgrade, e.g. `~/.cache` in `ro_home` becomes read-only) and widening (upgrade, e.g. re-declaring an installed path as read-write cache).
      - **Removal is not expressible**, by choice. Should a concrete need appear, it is one more class in the same table (`hide = [...]`, rendered as a tmpfs — the mechanism that already hides `$HOME`), not a new mechanism. Candidate, not built.
      - **User-global only.** A per-project `./.cave/profiles.toml` would sit in the read-write working directory, so an agent could widen its own boundary on the next run — accidental under this threat model, but a sandbox whose rules live inside the sandbox is the wrong shape. `~/.config/cave/` is hidden inside the sandbox, so the confining policy is unreadable from within. A per-project file would need an explicit opt-in flag.
      - **Parse failures are fatal and located.** A malformed user file exits non-zero naming file and line; ignoring it would run the default boundary while the user believed it was widened. The embedded table keeps `expect` — a build-time invariant covered by `test_load_config`.
      - `--doctor` prints the loaded sources and the effective merged table, answering why a path is or is not mounted.

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
- **Dogfood before widening** (decided 2026-10-22): Phase 3 adds the `agy` profile first, so cave's own development runs under cave, then the user config file, then the remaining profiles. Running the tool on itself exposes profile gaps faster than adding profiles for agents not in daily use, and the config file lands second so later profiles can be validated by editing a file instead of rebuilding.

## Packaging & Distribution

Decided 2026-10-23 at Phase 4 Decision Check Point:
- **Package Format**: Arch Linux VCS package (`cave-git`) selected over a static release package. At this stage, `cave` has no published release tags on GitHub; a VCS package tracks `main` dynamically via `pkgver()` (using `git describe` with commit count/hash fallback), allowing Arch Linux users to build and update immediately from source. Once stable semantic version tags are published, a companion `cave` release package using source tarballs and static checksums can be added alongside `cave-git`.
- **Placement**: Dedicated `packaging/arch/PKGBUILD` to isolate packaging metadata from core application code.
- **Dependencies**: Runtime dependency on `bubblewrap` (`bwrap`); build dependencies on `cargo` and `git`.
- **System Installation**: Installs binary to `/usr/bin/cave`, documentation to `/usr/share/doc/cave/README.md`, example configuration template to `/usr/share/doc/cave/profiles.toml.example`, and license to `/usr/share/licenses/cave/LICENSE`.
- **Clean Chroot Builds**: `build()` compiles only the executable (`--bin cave`), avoiding compilation of test binaries in release mode. `check()` executes unit tests (`cargo test --lib --bins`) to ensure test coverage without requiring unprivileged user namespace capabilities that are blocked in clean chroot build environments (e.g. `extra-x86_64-build`).

