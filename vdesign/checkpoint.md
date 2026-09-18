# Checkpoint

## Current State
- Phase 1 (Backend Spike & Decision) complete.
  - Spike script verified at [`vdesign/utils/spike-bwrap.sh`](utils/spike-bwrap.sh).
  - Boundary, terminal inheritance, and <10ms startup overhead validated.
  - Decisions recorded in [`docs/architecture_design.md`](../docs/architecture_design.md): `bwrap` backend, direct terminal inheritance, Rust CLI implementation, loose coupling, and `--dir /tmp` host tmp reuse.
  - Toolchain documented in [`vdesign/env.md`](env.md).
  - Ready for auditor review before Phase 2.

## Next Steps
- Awaiting auditor review.
- Upon clearance, begin Phase 2 (Walking Skeleton: `cave claude` in Rust with boundary tests first).
