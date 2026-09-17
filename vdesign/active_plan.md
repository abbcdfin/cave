# Active Plan: Phase 1 — Backend Spike & Decision

## Context & Objectives
Build an exploratory `bwrap` spike script in `vdesign/utils/spike-bwrap.sh` to validate the file boundary, inspect required home directory paths for `claude` and dev toolchains, and evaluate direct terminal inheritance (signals, resize, colors) against the selection criteria in `docs/architecture_design.md`.

## Tasks

- [x] **1.1: Host Environment Discovery**
  - Check `bwrap` installation and unprivileged user namespace availability.
  - Locate `claude` and examine its runtime dependencies (`which claude`, Node/nvm paths, env vars).
- [ ] **1.2: Home Directory Classification Matrix**
  - Identify and classify paths needed by `claude` and common toolchains:
    - Agent state (Read-Write, e.g. `~/.claude`, `~/.claude.json`)
    - Toolchain runtime (Read-Only, e.g. `~/.local/bin`, `~/.nvm`, etc.)
    - Hidden/Masked (e.g. `~/.ssh`, `~/.aws`, `~/.gnupg`, `~/.bashrc`, `/var/run/docker.sock`)
- [ ] **1.3: Throwaway Spike Implementation (`vdesign/utils/spike-bwrap.sh`)**
  - Construct minimal `bwrap` command mounting:
    - RO system mounts (`/usr`, `/etc`, `/lib`, `/bin`, etc.)
    - Private `/tmp`, `/run`, `/dev`, `/proc`
    - CWD mounted RW at its host path
    - Private/isolated home with selective bindings for agent state and toolchains
    - Network enabled, PID/IPC/UTS namespaces unshared
- [ ] **1.4: Terminal & Boundary Verification**
  - Test terminal behavior: window resizing (`SIGWINCH`), Ctrl-C, colors, interactive prompt.
  - Verify boundary: test write access outside CWD/agent state, verify host `/tmp` hidden, verify docker/systemd unreachable.
- [ ] **1.5: Decision Check Point & Architecture Update**
  - Compare results against criteria in `docs/architecture_design.md` (startup time, rootless, terminal fidelity, boundary enforcement).
  - Decide backend, implementation language, and terminal approach (direct vs relay).
  - Update `docs/architecture_design.md` and collapse active plan to complete Phase 1.
