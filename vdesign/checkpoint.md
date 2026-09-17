# Checkpoint

## Current State
- Scope decided 2026-09-17, recorded in [`docs/product_specification.md`](../docs/product_specification.md) › Scope: clumsy-agent threat model, file boundary, Linux only, no injected agent flags, `--dir` / `--ro-dir` / `--doctor`, allowed directories at host paths.
- Roadmap restructured into spike → walking skeleton → agent profiles in [`plan.md`](plan.md); rationale and backend selection criteria in [`docs/architecture_design.md`](../docs/architecture_design.md).
- No code yet.

## Next Steps
- Start Phase 1: build the `bwrap` spike in `vdesign/utils/` and measure it against the selection criteria.
