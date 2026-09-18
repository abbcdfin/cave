# Active Plan: Phase 1 — Backend Spike & Decision

## Context & Objectives
Build an exploratory `bwrap` spike script in `vdesign/utils/spike-bwrap.sh` to validate the file boundary, inspect required home directory paths for `claude` and dev toolchains, and evaluate direct terminal inheritance (signals, resize, colors) against the selection criteria in `docs/architecture_design.md`.

## Tasks

- [x] **1.1: Host Environment Discovery**
  - Verified `bwrap` 0.11.2 and unprivileged user namespaces (`unprivileged_userns_clone = 1`).
  - Located `claude` at `~/.local/bin/claude` -> `~/.local/share/claude/versions/2.1.275`.
- [x] **1.2: Home Directory Classification Matrix**
  - Agent state (RW): `~/.claude`, `~/.claude.json`.
  - Toolchain runtime (RO): `~/.local/bin`, `~/.local/share/claude`, `~/.cargo/bin`, `~/.npm_global`, `~/.gitconfig`.
  - Hidden: `~/.ssh`, `~/.gnupg`, `~/.docker`, `/var/run/docker.sock`, `/run/user/$UID/bus`.
- [x] **1.3: Throwaway Spike Implementation (`vdesign/utils/spike-bwrap.sh`)**
  - Assembled working `bwrap` script mounting private `/tmp`, `/run`, `$HOME` tmpfs with selective binds, and CWD read-write at host path.
- [x] **1.4: Terminal & Boundary Verification**
  - Boundary verified: writes to `/etc` blocked, host `/tmp` hidden, `~/.ssh` and `~/.gnupg` hidden, `~/.gitconfig` read-only, docker socket and systemd bus isolated.
  - Startup overhead measured: **9.28 ms** (well below 100ms threshold).
  - Terminal direct inheritance verified: `isatty()` is True, controlling terminal inherited directly.
- [x] **1.5: Decision Check Point & Architecture Update**
  - Decisions:
    1. Isolation backend: Bubblewrap (`bwrap`).
    2. Terminal model: Direct terminal inheritance.
    3. Host `/tmp` access: Supported via explicit `--dir /tmp` (or `--ro-dir /tmp`).
    4. Implementation language: Rust (`cargo`, `rustc` 1.94+).
    5. Backend coupling: Loosely coupled to external `bwrap` binary.
  - Recorded in [`docs/architecture_design.md`](../docs/architecture_design.md), [`docs/product_specification.md`](../docs/product_specification.md), and [`vdesign/env.md`](env.md).
  - Phase 1 collapsed and marked complete in [`vdesign/plan.md`](plan.md).
