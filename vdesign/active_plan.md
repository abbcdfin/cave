# Active Plan — Phase 6: Toolchains (`cave --with flutter`) (Reopened)

Status: BUILDING. Exit criteria: see `vdesign/plan.md` › Phase 6 ("Done when").
Decisions & rationale: `docs/architecture_design.md` › Toolchains, argument in `vdesign/proposals/toolchain-profiles.md` at `b5015eb`.

## Slice 1: Single Resolver & Engine Boundary Check (Items 1a, 2, 3)
- [x] 6.6.1 Implement `EntryResolution` enum and unified `resolve_toolchain` in `src/policy.rs`: expand, check existence, and classify path validity (fatal `InvalidRootOrHome` when path is `/`, `$HOME`, or a folder containing $HOME).
- [x] 6.6.2 Delete `resolve_path_literal` and `resolve_literal_toolchain` in `src/policy.rs`, updating tests to use the real resolver.
- [x] 6.6.3 Update `src/main.rs` to use `resolve_toolchain`: fatal exit 1 on `InvalidRootOrHome` naming toolchain, entry, and resolved path (without `--dir` warning); fatal exit 1 on failed `requires`; skip warning on unresolvable/missing `rw_cache`/`runtime`.
- [x] 6.6.4 Update `run_doctor` in `src/cli.rs` to use `resolve_toolchain`, displaying `[FAIL]` for `InvalidRootOrHome`.
- [x] 6.6.5 Unit tests for 1a in `src/policy.rs`: entry resolving to `/`, `$HOME`, and an ancestor folder containing $HOME.

## Slice 2: Flutter Sentinel & Host-Independent Binary Lookup (Items 1b, 4)
- [x] 6.7.1 Update `[toolchains.flutter]` in `src/profiles.toml`: add `"${bin:flutter}/../../packages/flutter_tools"` to `requires`.
- [x] 6.7.2 Update `which_binary` in `src/bwrap.rs` to only fall back to `/usr/bin` when `PATH` is unset, and decouple missing-flutter boundary test from host binaries.
- [x] 6.7.3 Add boundary test for 1b: plain script at `$HOME/bin/flutter` in fake home exits 1 and agent never runs.
- [x] 6.7.4 Add boundary test for 1a with no sentinel: toolchain in user config with `rw_cache = ["${bin:faketool}/../.."]` exits 1.
- [x] 6.7.5 Mutation checks for 1a and 1b separately, recording which test catches each (1a: `test_toolchain_flutter_boundary_and_isolation` Case 5; 1b: Case 4).

## Slice 3: Documentation, Plan & Checkpoint Upkeep (Items 1a, 5, 6)
- [x] 6.8.1 Reconcile `docs/architecture_design.md` › Toolchains: document fatal exit 1 on `/`, `$HOME`, or ancestor containing `$HOME`, removing the previous claim of sharing `--dir` warnings.
- [x] 6.8.2 Correct `vdesign/plan.md` and `vdesign/checkpoint.md`: name `ExpansionLookup`, clarify `build_mount_plan` non-purity, clarify 0.75 ms startup measurement, reference proposal at `b5015eb`, keep Phase 6 as `[/]` awaiting device verification.
- [x] 6.8.3 Verify all tests pass (`cargo test`) and clippy is clean (`cargo clippy --all-targets -- -D warnings`).
