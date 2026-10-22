# Project Plan — cave

## Roadmap

Rationale: `docs/architecture_design.md` › Delivery Approach.

- [x] Phase 1: Backend Spike & Decision
  - Outcome: Bubblewrap (`bwrap`) selected as isolation backend (9.28 ms startup overhead, rootless, clean tmpfs masking). Direct terminal inheritance selected (native signals/TTY passthrough, no relay). Rust selected as CLI implementation language (<2ms overhead, single static binary, `CommandExt::exec` process replacement). Loosely coupled to external `bwrap`. Reusing host `/tmp` supported via `--dir /tmp`. Spike script preserved at `vdesign/utils/spike-bwrap.sh`.
- [x] Phase 2: Walking Skeleton — `cave claude`
  - Outcome: Implemented in Rust (`src/cli.rs`, `src/policy.rs`, `src/bwrap.rs`). Embedded profiles (`src/profiles.toml`) with Claude Code profile (RW state `~/.claude/`, `~/.claude.json`, RO runtime `~/.local/share/claude`). Mount order rule enforces defaults → profile → CWD → `--dir` / `--ro-dir` overrides. Private tmpfs for `/tmp`, `/run`, `/var/tmp`, and `$HOME`. Sensitive paths hidden. Host environment sanitized (`SSH_AUTH_SOCK`, `DBUS_SESSION_BUS_ADDRESS` unset). Direct terminal inheritance via `execve`. Hardened boundary tests (12 unit, 11 boundary tests green), typed state entries with valid JSON seeding (`init = "{}"`), and raw OsString passthrough. Initial `--doctor` diagnostics check bwrap availability and AppArmor unprivileged user namespace sysctls; sandbox DNS and `/var/run` symlink checks deferred to Phase 3. Live interactive verification confirmed on host terminal (direct TTY, SIGWINCH resize, colors, Ctrl-C, Ctrl-Z/fg); a setting changed inside `cave claude` (`/config`) persisted directly to native host `claude`, confirming live host state writes on existing inode without triggering kernel mount-point `EBUSY`. Added startup overhead measured at 7.37 ms (budget < 100ms).
- [/] Phase 3: Agent Profiles & Diagnostics
  - Sequenced for dogfooding (decided 2026-10-22, `docs/architecture_design.md` › Delivery Approach): `agy` profile, then the user config file, then the remaining profiles.
  - Add the `agy` profile as data (`~/.gemini`); precondition resolved 2026-10-22 — `agy` is a terminal CLI, `/usr/bin/antigravity` is the GUI launcher and is out of scope.
  - Warn when no profile matches the agent basename: ephemeral `$HOME`, state will not persist, `--dir` as the escape hatch.
  - Layer a user config file (`~/.config/cave/profiles.toml`) over the embedded table: append semantics, located parse errors, user-global only.
  - Refine profiles: express a read-only subpath inside a read-write state directory, then add Aider and a generic CLI profile.
  - Implement `cave --doctor`, including the loaded config sources, the effective merged table, and the sandbox DNS and `/var/run` checks deferred from Phase 2.
  - Exit: every supported agent runs under cave with no engine change.

## Candidates
<!-- Unnumbered DRAFT ideas awaiting validation -->
- Candidate: Network policy filtering (allowlist API endpoints, block local LAN/intranet).
- Candidate: Ephemeral git branch / copy-on-write snapshotting before agent execution.
- Candidate: Custom target paths (`--dir SRC:DEST`, `--workdir DEST`). Deferred 2026-09-17 in favour of host paths; revisit only for a concrete need. Known costs: breaks relative paths between directories (`/work/../lib` lands on `/lib`), path-keyed agent state (`claude -c`), and absolute paths inside projects (venvs, worktrees).
- Candidate: Opt-in Docker access for projects whose agents need `docker compose`; the socket is hidden by default (see `docs/product_specification.md` › Scope).
- Candidate: `hide = [...]` class in the profile table, so a user file can remove an inherited default instead of only downgrading it. Deferred 2026-10-22 until a concrete need; it renders as a tmpfs mount, the mechanism that already hides `$HOME`.
- Candidate: Per-project profile file (`./.cave/profiles.toml`) behind an explicit opt-in flag. Deferred 2026-10-22: the working directory is agent-writable, so the file could widen the boundary it defines.
- Candidate: `--profile NAME` CLI option to override automatic basename profile detection.
