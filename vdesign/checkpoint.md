# Checkpoint

## Current State
- Phase 2 (Walking Skeleton — `cave claude`) is complete:
  - Rust implementation (`cli`, `policy`, `bwrap`, `profiles.toml`).
  - Hardened boundary test suite (12 unit tests, 11 boundary tests passing).
  - All 4 core remediation bugs fixed (mount ordering, `$HOME` CWD warning, atomic state writes with JSON seeding, first-run state auto-creation).
  - Clippy clean (0 warnings).
  - Benchmark overhead verified at 7.37 ms (budget < 100 ms).
  - Direct execution verified against host `claude` (version and session invocation inside sandbox).
  - Phase 2 remediation committed in commit `70eba8f`.

## Next Steps
- Transition to Phase 3: Agent Profiles & Diagnostics.
  - Populate `vdesign/active_plan.md` with granular tasks for Phase 3:
    1. Agent Profiles data (Aider, Antigravity `agy`, generic CLI).
    2. Unknown agent warning verification & test coverage.
    3. User config layering (`~/.config/cave/profiles.toml` append semantics).
    4. Diagnostic display in `cave --doctor` (config sources and merged effective table).
