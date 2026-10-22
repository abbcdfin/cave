# Active Plan: Phase 2 Remediation — Walking Skeleton (`cave claude`)

## Context & Objectives
Address auditor review findings: harden the boundary test suite so tests fail on real violations, fix the 4 confirmed bugs, resolve edge cases (non-UTF-8, missing HOME, path comparisons), and complete manual terminal evidence.

## Tasks

- [x] **2.R1: Test Harness Hardening**
  - Automatically injected `set -eu` into the test runner in `tests/boundary_tests.rs`.
  - Rewrote negative assertions with explicit `if <cmd>; then exit 1; fi` checks and verified host state.
  - Asserted host test runner PID is absent from `/proc/$HOST_PID` (`test_process_namespace_isolation`).
  - Added process termination test verifying killing `cave` kills child (`test_killing_cave_kills_agent_process`).
  - Added harness verification ensuring failure to reach sentinel or non-zero exit is caught (`test_harness_catches_failure`).
- [x] **2.R2: Fix 4 Core Bugs**
  - **Bug 1 (Mount order)**: Unified `--dir` and `--ro-dir` into ordered `ExtraDir` vector in `cli.rs` and `policy.rs`. Verified by `test_command_line_mount_order_respected`.
  - **Bug 2 (`$HOME` as CWD)**: Added warning comparing canonicalized CWD against canonicalized `$HOME` and `/`.
  - **Bug 3 (State file rename / EBUSY)**: Documented Linux VFS single-file bind mount point rename constraint (`EBUSY` on atomic replace) in `docs/architecture_design.md` and `src/policy.rs`. Verified Claude Code in-place writes (`O_WRONLY|O_TRUNC`) succeed on existing inode and update host mtime.
  - **Bug 4 (First-run persistence)**: Added `prepare_host_state` in `src/policy.rs` to auto-create declared agent state directories and files on host with typed seeding before mounting. Verified by `test_first_run_persistence_creates_state`.
- [x] **2.R3: Fix Smaller Items**
  - Switched `env::args()` to `env::args_os()` in `main.rs` to handle non-UTF-8 arguments safely.
  - Returned a clean error when `$HOME` is unset instead of falling back to `/`.
  - Canonicalized paths before checking `--dir /` and `--dir ~` warnings.
  - Fixed all clippy warnings (0 warnings in `cargo clippy --all-targets`).
  - Implemented initial `cave --doctor` diagnostics in `cli.rs` and `main.rs` (bwrap availability, AppArmor sysctl).
- [ ] **2.R4: Exit Criteria & Manual Evidence**
  - [x] Automated tests verified: 12 unit tests and 11 boundary tests pass (`cargo test`).
  - [x] Benchmark verified: 7.37 ms added overhead (budget < 100 ms).
  - [ ] Manual terminal verification on host (requires human at interactive terminal):
    - [ ] Run `cave claude --dangerously-skip-permissions`
    - [ ] Verify terminal resize (`SIGWINCH`), colors, Ctrl-C (`SIGINT`), and Ctrl-Z / `fg` (`SIGTSTP`)
    - [ ] Change a setting inside the session (e.g. via `/config`), exit, and verify native `claude` on the host reflects the change

