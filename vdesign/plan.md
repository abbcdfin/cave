# Project Plan — cave

## Roadmap

Rationale: `docs/architecture_design.md` › Delivery Approach.

- [/] Phase 1: Backend Spike & Decision
  - Build a throwaway `bwrap` spike in `vdesign/utils/` that runs `claude` inside the boundary from `docs/product_specification.md` › Scope.
  - Measure it against the selection criteria in `docs/architecture_design.md` › Isolation Engine.
  - List the home paths agents and their toolchains need (e.g. `~/.local/bin/claude`, `~/.claude/`, `~/.cargo`, `~/.nvm`, `~/.cache`) and classify each as agent state (RW), runtime (RO), or hidden.
  - Check the host terminal used directly: resize, Ctrl-C, Ctrl-Z, colours.
  - Exit: Decision Check Point on backend, implementation language, and terminal approach (direct or PTY relay), recorded in `docs/architecture_design.md`.
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
