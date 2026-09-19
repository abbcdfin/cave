# Checkpoint

## Current State
- Completed Phase 2 (Walking Skeleton: `cave claude`).
  - Rust crate implemented (`cli`, `policy`, `bwrap`, `main`).
  - Embedded profile data in [`src/profiles.toml`](../src/profiles.toml).
  - All 11 unit tests and 8 hermetic boundary integration tests passing.
  - Startup overhead verified: 7.36 ms (limit: 100 ms).
  - Documentation updated and synchronized with implementation.

## Next Steps
- Phase 3: Agent Profiles & Diagnostics
  - Verify `agy` invocation characteristics (terminal agent vs launcher).
  - Add Aider, Antigravity, and generic CLI profiles to `src/profiles.toml`.
  - Implement `cave --doctor` diagnostics.
