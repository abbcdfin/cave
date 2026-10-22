# Checkpoint

## Current State
- Phase 2 (Walking Skeleton — `cave claude`) is reopened pending Task 2.R4 manual terminal evidence:
  - Engineering fixes verified: 12 unit tests and 11 boundary tests passing (`cargo test`), 0 clippy warnings.
  - Mount ordering preserved, canonicalized CWD warning implemented, raw `OsString` args preserved, first-run state auto-creation with valid JSON seeding verified.
  - Documented Linux VFS single-file bind mount point rename constraint (`EBUSY`) in `docs/architecture_design.md` and `src/policy.rs`.
  - Clarified `--doctor` scope (DNS check and `/var/run` symlink check scheduled for Phase 3) in `docs/architecture_design.md`.

## Next Steps
- Execute Task 2.R4 manual terminal verification at an interactive host terminal:
  1. `target/release/cave claude --dangerously-skip-permissions`
  2. Verify terminal resize (`SIGWINCH`), colors, Ctrl-C (`SIGINT`), and Ctrl-Z / `fg` (`SIGTSTP`).
  3. Change a setting inside the session (e.g. `/config`), exit, and verify native `claude` reflects the updated setting on the host.
- Once verified, mark 2.R4 complete in `vdesign/active_plan.md`, mark Phase 2 `[x]` in `vdesign/plan.md`, and collapse Phase 2.
