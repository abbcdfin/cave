# Checkpoint

## Current State
- Phase 2 (Walking Skeleton — `cave claude`) is complete and collapsed:
  - Rust implementation (`cli`, `policy`, `bwrap`, `profiles.toml`).
  - Hardened boundary test suite (12 unit tests, 11 boundary tests passing).
  - All review bugs fixed (mount ordering, `$HOME` CWD warning, raw `OsString` args, first-run state auto-creation with valid JSON seeding).
  - Clippy clean (0 warnings), 7.37 ms startup overhead (< 100 ms budget).
  - Linux VFS single-file bind mount point rename constraint (`EBUSY`) documented in `docs/architecture_design.md` and `src/policy.rs`.
  - Manual exit verification (2.R4) confirmed on host terminal: interactive TTY, resize, signals, and Claude Code setting persistence across sessions.
  - Phase 2 marked `[x]` in `vdesign/plan.md`.

## Next Steps
- Software Architect planning Phase 3 (Agent Profiles & Diagnostics):
  - Draft granular Phase 3 board in `vdesign/active_plan.md`.
  - Review with user before execution.
