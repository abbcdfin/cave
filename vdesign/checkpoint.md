# Checkpoint

## Current State
- Phase 6 (Toolchains, `cave --with flutter`) reopened for boundary hole fix and pending live device verification:
  - Fixed boundary hole: Any toolchain entry that resolves to `/`, `$HOME`, or an ancestor directory containing `$HOME` is fatal, exiting 1 with an error naming the toolchain, entry, and resolved path (without `--dir` warning).
  - Added sentinel `"${bin:flutter}/../../packages/flutter_tools"` to `[toolchains.flutter].requires` in `src/profiles.toml`, ensuring an isolated `flutter` binary outside an SDK exits 1 before anything is mounted.
  - Unified resolver: Created `resolve_entry` and `resolve_toolchain` in `src/policy.rs` as the single source of truth for expansion, existence, and classification (`EntryResolution`), used by both `main.rs` and `run_doctor` (which prints `[FAIL]` on `InvalidRootOrHome`).
  - Deleted dead helper functions `resolve_path_literal` and `resolve_literal_toolchain`.
  - Updated `which_binary` in `src/bwrap.rs` to only search explicit `PATH` when set, avoiding host `/usr/bin` leakage in tests.
  - Added unit test `test_resolve_entry_invalid_root_and_home` and boundary test cases in `tests/boundary_tests.rs` for 1a and 1b. Mutation checks verified separately for 1a (Case 5) and 1b (Case 4).
  - Test suite: 36 unit tests and 21 boundary tests passing (57 total), 0 clippy warnings (`-D warnings`).
  - Measured startup overhead: Total startup overhead for `cave --with flutter true` is 10.79 ms avg (min 9.65 ms); toolchain resolution adds ~0.75 ms vs base execution, well within the 100 ms budget.
  - Reconciled `docs/architecture_design.md` › Toolchains.
  - Plan / checkpoint corrections: Trait name is `ExpansionLookup`, `build_mount_plan` checks paths on disk and prints working directory warning, startup overhead clarified.

## Next Steps
- Awaiting user live device verification on host:
  1. `cave --with flutter sh -c 'cd /home/jian/workspace/sys-in/ble-mesh/raft_mesh_manager && flutter build apk --debug'`
  2. `cave --with flutter sh -c 'cd /home/jian/workspace/sys-in/ble-mesh/raft_mesh_manager && flutter run'` (or under `cave --with flutter claude`) with hot reload on a connected phone.
- Record live results as reported by user, mark Phase 6 `[x]`, and record verified outcome in `vdesign/plan.md`.
