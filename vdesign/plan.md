# Project Plan — cave

## Roadmap

Rationale: `docs/architecture_design.md` › Delivery Approach.

- [x] Phase 1: Backend Spike & Decision
  - Outcome: Bubblewrap (`bwrap`) selected as isolation backend (9.28 ms startup overhead, rootless, clean tmpfs masking). Direct terminal inheritance selected (native signals/TTY passthrough, no relay). Rust selected as CLI implementation language (<2ms overhead, single static binary, `CommandExt::exec` process replacement). Loosely coupled to external `bwrap`. Reusing host `/tmp` supported via `--dir /tmp`. Spike script preserved at `vdesign/utils/spike-bwrap.sh`.
- [ ] Phase 2: Walking Skeleton — `cave claude`
  - Write the boundary tests first: reads and writes outside allowed directories, symlinks pointing outside, `git config --global`, writes under `--ro-dir` and host runtime, host `/tmp` visibility, `docker ps` and `systemctl --user` reaching the host, ownership of files created in allowed directories.
  - Implement option parsing, `--dir` / `--ro-dir`, host-path mounts, hidden home, private `/tmp` and `/run`, read-only host runtime, and the Phase 1 terminal approach.
  - Define the agent profile format; Claude Code is the first profile.
  - Exit: boundary tests pass; `cave claude --dangerously-skip-permissions` and `cave claude -c` behave as on the host; added startup < 100ms.
- [ ] Phase 3: Agent Profiles & Diagnostics
  - Add Aider, Antigravity, and generic CLI profiles as data only; first confirm `agy` is a terminal agent rather than a GUI launcher.
  - Implement `cave --doctor`.
  - Exit: every supported agent runs under cave with no engine change.

## Candidates
<!-- Unnumbered DRAFT ideas awaiting validation -->
- Candidate: Network policy filtering (allowlist API endpoints, block local LAN/intranet).
- Candidate: Ephemeral git branch / copy-on-write snapshotting before agent execution.
- Candidate: Config file for extra allowed directories, complementing the CLI options.
- Candidate: Custom target paths (`--dir SRC:DEST`, `--workdir DEST`). Deferred 2026-09-17 in favour of host paths; revisit only for a concrete need. Known costs: breaks relative paths between directories (`/work/../lib` lands on `/lib`), path-keyed agent state (`claude -c`), and absolute paths inside projects (venvs, worktrees).
- Candidate: Opt-in Docker access for projects whose agents need `docker compose`; the socket is hidden by default (see `docs/product_specification.md` › Scope).
