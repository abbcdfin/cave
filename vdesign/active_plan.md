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
  - **Bug 3 (State file rename / EBUSY)**: Verified Claude Code's in-place write persistence on host (`mtime` updated); documented kernel mount point rename constraint.
  - **Bug 4 (First-run persistence)**: Added `ensure_host_state_path_exists` in `src/policy.rs` to auto-create declared agent state directories and files on host before mounting. Verified by `test_first_run_persistence_creates_state`.
- [x] **2.R3: Fix Smaller Items**
  - Switched `env::args()` to `env::args_os()` in `main.rs` to handle non-UTF-8 arguments safely.
  - Returned a clean error when `$HOME` is unset instead of falling back to `/`.
  - Canonicalized paths before checking `--dir /` and `--dir ~` warnings.
  - Fixed all clippy warnings (0 warnings in `cargo clippy`).
  - Implemented `cave --doctor` diagnostics in `cli.rs` and `main.rs`.
- [ ] **2.R4: Exit Criteria & Manual Evidence**
  - Automated tests verified: 10 unit tests and 10 boundary tests pass (`cargo test`).
  - Benchmark verified: 7.37 ms added overhead (budget < 100 ms).
  - Awaiting manual terminal verification on host (`cave claude --dangerously-skip-permissions` and `cave claude -c`).
