# Checkpoint

## Current State
- Phase 2 (Walking Skeleton — `cave claude`) complete and collapsed into `plan.md`.
- Phase 3 (Agent Profiles & Diagnostics) accepted and under construction; board in
  [`vdesign/active_plan.md`](active_plan.md), sequenced for dogfooding: `agy` profile (3.1),
  unknown-agent warning (3.2), user config file layering (3.3), profile refinement (3.4),
  `--doctor` completion (3.5).
- Phase 3 precondition resolved 2026-10-22: `agy` is a terminal CLI with state under `~/.gemini`;
  `/usr/bin/antigravity` is the GUI launcher, and `~/.antigravity` and `~/.config/Antigravity`
  are the IDE's directories, not the CLI's.

## Next Steps
- Developer executes 3.1: `[profiles.agy]` with `state = ~/.gemini`, boundary test, and live
  verification that a session resumes in both directions between host and cave.
- Open engine decision deferred to 3.4: a read-only subpath inside a read-write state directory
  (`~/.gemini/antigravity-cli/bin`) is not expressible while `runtime` renders before `state`.
- Review each slice before it is collapsed.
