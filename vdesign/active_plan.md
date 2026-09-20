# Active Plan: Phase 2 Remediation — Walking Skeleton (`cave claude`)

## Context & Objectives
Address auditor review findings: harden the boundary test suite so tests fail on real violations, fix the 4 confirmed bugs, resolve edge cases (non-UTF-8, missing HOME, path comparisons), and complete manual terminal evidence.

## Tasks

- [ ] **2.R1: Test Harness Hardening**
  - Inject `set -eu` into the test runner automatically.
  - Rewrite negative assertions with explicit `if <cmd>; then exit 1; fi` checks and host-side verification.
  - Test PID isolation by asserting host test runner PID is absent from `/proc/$HOST_PID`.
  - Add test that killing `cave` terminates the child process.
  - Add harness verification ensuring deliberate boundary violations fail tests.
- [ ] **2.R2: Fix 4 Core Bugs**
  - **Bug 1 (Mount order)**: Preserve command-line order between `--dir` and `--ro-dir` in `cli.rs` and `policy.rs`.
  - **Bug 2 (`$HOME` as CWD)**: Forbid or safely handle running `cave` from `$HOME` to prevent exposing the host home directory.
  - **Bug 3 (State file rename / EBUSY)**: Resolve single-file mount rename failure for `~/.claude.json`.
  - **Bug 4 (First-run persistence)**: Auto-create declared agent state paths on the host before mounting if they do not exist yet.
- [ ] **2.R3: Fix Smaller Items**
  - Switch `env::args()` to `env::args_os()` in `main.rs` to handle non-UTF-8 arguments safely.
  - Return a clean error when `$HOME` is unset instead of falling back to `/`.
  - Canonicalize paths before checking `--dir /` and `--dir ~` warnings.
  - Fix all clippy warnings.
  - Align README `--doctor` mention and record notes on candidate per-project configs.
- [ ] **2.R4: Exit Criteria & Manual Evidence**
  - Verify all hardened boundary tests pass.
  - Record manual interactive terminal checks (`cave claude --dangerously-skip-permissions` and `cave claude -c`: resize, Ctrl-C, Ctrl-Z, colors, setting persistence).
