# Checkpoint

## Current State

- Phases 1–8 are DONE; outcomes and verification are in [the plan](plan.md#increments).
  Released [v0.2.0](../CHANGELOG.md#v020--2026-11-10) and
  [v0.3.0](../CHANGELOG.md#v030--2026-11-19); `main` and `origin/main` are at `v0.3.0`.
- Onboarded to vibe 0.12.0 (2026-11-25): `dev` created from `main`, the `pre-push` hook
  installed, `vdesign/env.md` renamed `dev_env.md`, `constraints.md` reduced to project
  limits, `CHANGELOG.md` backfilled, and release steps moved to the `dev` → `main` model
  (`docs/architecture_design.md` › Versioning › Branches). `dev` is local only.
- No increment is BUILDING. 66 tests (43 unit, 23 boundary) and clippy (`-D warnings`)
  pass at `v0.3.0`.

## Next Steps

- Push `dev` to `origin` (needs the user's approval).
- Publish `cave-git` to the AUR.
- Choose increment 9 from the candidates in `plan.md`. The "Generic CLI fallback
  profile" candidate looks covered by Phase 8's `--profile`; confirm or reject it.
