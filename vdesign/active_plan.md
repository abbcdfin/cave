# Active Plan: Phase 2 — Walking Skeleton (`cave claude`)

## Context & Objectives
Build the thinnest real `cave`: a Rust binary that merges engine defaults, the Claude Code agent profile, the working directory, and `--dir` / `--ro-dir` into one ordered mount plan, then execs `bwrap` with it, using the host terminal directly.

- Boundary: `docs/product_specification.md` › Scope.
- Policy (mount order, path classes, environment, agent profiles): `docs/architecture_design.md` › Filesystem & Environment Policy.
- Exit (from `plan.md`): boundary tests pass; `cave claude --dangerously-skip-permissions` and `cave claude -c` behave as on the host; added startup < 100ms.

## Tasks

- [ ] **2.0: De-risk the Phase 1 decisions**
  - Re-run the spike after the review fixes: `cargo --version`, `claude --version`, name resolution, `/var/tmp` writable, `/var` read-only.
  - Run `claude` interactively in the spike: resize, Ctrl-C, Ctrl-Z then `fg`, colours, and `claude -c` resuming a session started on the host.
  - If resize or Ctrl-Z fails, stop: the terminal decision returns to a Decision Check Point before code depends on it.
- [ ] **2.1: Crate scaffold**
  - Binary crate at the repo root (`Cargo.toml`, `src/`, `tests/`) with modules `cli`, `policy`, `bwrap`.
  - Tests that need `bwrap` fail with a clear message when it is missing; a silently skipped boundary test reads as a pass.
- [ ] **2.2: Boundary tests — written before 2.3–2.5, expected red**
  - Hermetic fixture: a temp home holding fake `~/.ssh`, `~/.claude`, `~/.cache`, `~/.config`, a sibling project, and a marker in host `/tmp`. Tests point `HOME` at it and never touch the real home.
  - Outside allowed directories: fake `~/.ssh`, `~/.config`, and the sibling project are invisible; writes there never reach the host; `ls ~` lists only allowed entries.
  - Symlinks: a link in the working directory pointing at fake `~/.ssh` dangles.
  - Read-only: writes fail under `/usr`, `/etc`, `/var`, `--ro-dir` paths, and to `~/.gitconfig` (`git config --global`).
  - Read-write: writes to the working directory, `--dir` paths, agent state, and `~/.cache` land on the host, owned by the host user.
  - Private: the host `/tmp` marker is invisible; sandbox `/tmp` and `/var/tmp` writes never reach the host; `/run/docker.sock` and `/var/run/docker.sock` are absent; `/run/user/$UID` exists, empty, mode `0700`.
  - Overrides: `--dir /tmp` shows the host marker; `--ro-dir .` makes the working directory read-only.
  - Environment: a host test variable passes through; `SSH_AUTH_SOCK` and `DBUS_SESSION_BUS_ADDRESS` are unset; `XDG_RUNTIME_DIR` is `/run/user/$UID`.
  - Processes: host PIDs are invisible; killing the `cave` process kills the agent.
- [ ] **2.3: CLI**
  - `cave [cave-options] <agent> [args...]`: option parsing stops at the first non-option word, and everything from the agent command on passes through unchanged (`cave claude --dir x` hands `--dir x` to `claude`).
  - Options: `--dir PATH` and `--ro-dir PATH` (repeatable), `--help`, `--version`. `--doctor` is Phase 3.
  - Errors: unknown option, missing agent command, nonexistent `--dir` / `--ro-dir` path. Warnings: `--dir /`, `--dir ~`.
  - Unit tests for each rule.
- [ ] **2.4: Policy engine**
  - Engine defaults and agent profiles as data embedded in the binary (TOML, parsed at startup), in a schema the config-file candidate can reuse.
  - Claude Code profile: state `~/.claude/`, `~/.claude.json` (RW); runtime `~/.local/share/claude` (RO).
  - Profile selection by the agent command's basename; an unknown command gets the engine defaults only.
  - Host path resolution: relative paths against the working directory; symlinks resolved, so a symlinked working directory lands where native `getcwd()` puts it; default paths that do not exist are skipped.
  - Output: one ordered mount plan (defaults → profile → working directory → `--dir` / `--ro-dir` in command-line order) plus the environment plan. Pure function, unit-tested without `bwrap`.
- [ ] **2.5: `bwrap` backend**
  - Render the plan to `bwrap` arguments (namespaces and `--die-with-parent` as in the spike), locate `bwrap` on `PATH`, and `exec` it; cave does not stay resident.
  - Missing `bwrap`: an error naming the package to install.
- [ ] **2.6: Exit checks**
  - 2.2 boundary tests green.
  - Manual, in this repo: `cave claude --dangerously-skip-permissions` and `cave claude -c`; resize, Ctrl-C, Ctrl-Z, colours.
  - Startup: `cave /usr/bin/true` against bare `/usr/bin/true`, averaged over 20 runs, recorded (budget < 100ms; the spike measured 9.16 ms).
- [ ] **2.7: Sync docs and collapse**
  - Replace the path lists in `docs/architecture_design.md` with a pointer to the embedded data file, keeping the classes and their reasons there, so each list lives in one place.
  - README examples match the built behaviour.
  - Collapse this board into `plan.md`.
