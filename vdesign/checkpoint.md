# Checkpoint

## Current State

- Phases 1–8 are DONE; outcomes and verification are in [the plan](plan.md#increments).
  Released [v0.2.0](../CHANGELOG.md#v020--2026-11-10) and
  [v0.3.0](../CHANGELOG.md#v030--2026-11-19); `main` and `origin/main` are at `v0.3.0`.
- Onboarded to vibe 0.12.0 (2026-11-25): `dev` created from `main`, the `pre-push` hook
  installed, `vdesign/env.md` renamed `dev_env.md`, `constraints.md` reduced to project
  limits, `CHANGELOG.md` backfilled, and release decisions recorded in
  [ADR 0001](../docs/adr/0001-releases.md). `dev` is on `origin`.
- No increment is BUILDING. 66 tests (43 unit, 23 boundary) and clippy (`-D warnings`)
  pass at `v0.3.0`.

## Next Steps

- Publish `cave-git` to the AUR.
- Choose increment 9 from the candidates in `plan.md`.
- At the next release, add vibe's pre-publish upgrade check to the README's release
  steps: a package built from a local copy of the release commit, upgraded from the
  installed version by following only `CHANGELOG.md`.
