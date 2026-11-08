# Active Plan — Phase 6: Toolchains (`cave --with flutter`)

Status: BUILDING. Exit criteria: see `vdesign/plan.md` › Phase 6 ("Done when").
Decisions & rationale: `docs/architecture_design.md` › Toolchains, `vdesign/proposals/toolchain-profiles.md`.

## Slice 1: Thin Path — Config, Selection & Literal Mounts (Plan step 1)
- [x] 6.1.1 Config data models: Add `toolchains` table to `ConfigData` and `UserConfigData` in `src/policy.rs` (`ToolchainConfig`, `UserToolchainConfig`), update `load_config()`, and implement `merge_user_config` layering (append and move between `rw_cache`/`runtime`).
- [x] 6.1.2 CLI argument parsing: Add `--with NAME` option in `src/cli.rs` (repeatable, parsed for both `Run` and `Doctor` commands).
- [x] 6.1.3 Mount plan & validation: Validate `--with` names against config in `src/main.rs` (unknown name exits 1); mount toolchains in `build_mount_plan` after agent profile (step 8) and before working directory (step 9) in `--with` order, rendering `rw_cache` before `runtime` without creating paths.
- [x] 6.1.4 Tests for Slice 1: Unit tests for toolchain config parsing, user config merging, `--with` CLI parsing, and literal toolchain mounting order.

## Slice 2: Expansion & Purity (Plan step 2)
- [x] 6.2.1 Pure expansion function in `src/policy.rs` over a lookup trait/closure supporting `${VAR}`, `${VAR:-default}`, and `${bin:NAME}` (using `bwrap::which_binary`, `fs::canonicalize` on binary, then lexical normalization for `..`).
- [x] 6.2.2 Resolution & validation in `src/main.rs`: Resolve toolchain entries in `main.rs`; exit 1 naming missing entry if a `requires` entry fails to resolve; emit single stderr warning naming toolchain and entry for non-`requires` unresolved entries and skip them; check resolved paths with `cli::check_warning` (`/` and `$HOME`). Pass resolved toolchains to `build_mount_plan`.
- [x] 6.2.3 Tests for Slice 2: Unit tests covering each expansion form (variable set, unset, `:-default`, missing binary, symlinked binary, `..` resolution), requires exit condition, unresolvable non-requires warning, and purity verification.

## Slice 3: TCP Diagnostics & Doctor (Plan step 3)
- [ ] 6.3.1 `check_tcp` support: Parse `check_tcp = [{ addr, hint }]` in `ToolchainConfig`; implement non-blocking/short-timeout TCP connect check (`TcpStream::connect_timeout` tens of ms) in `src/policy.rs` / `src/main.rs` and warn on failure with hint at startup.
- [ ] 6.3.2 Doctor integration: Update `cave --doctor` to accept `--with NAME` and report effective toolchain entries, resolved paths, and `check_tcp` connectivity status.
- [ ] 6.3.3 Measure startup overhead with `--with flutter` ensuring it remains < 100 ms.

## Slice 4: Built-in Flutter Toolchain & Boundary Tests (Plan step 4)
- [ ] 6.4.1 Embedded `[toolchains.flutter]` in `src/profiles.toml` with `requires`, `rw_cache` (`${bin:flutter}/../..`, `.pub-cache`, `.gradle/caches`, `.gradle/wrapper`, `.dartServer`), `runtime` (`${ANDROID_HOME:-Android/Sdk}`, `.android/debug.keystore`, `.config/flutter`), and `check_tcp` for adb (127.0.0.1:5037).
- [ ] 6.4.2 Boundary tests in `tests/boundary_tests.rs`: Fake-toolchain fixture on `PATH` verifying SDK writable, runtime read-only, `.gradle/gradle.properties` and `.android/adbkey` hidden, missing `flutter` exits 1, unknown `--with` exits 1.
- [ ] 6.4.3 Mutation check: Temporarily disable fatal check for failed `requires`, verify boundary test catches it, and restore.
- [ ] 6.4.4 Host verification: Provide user with exact commands for host checks (`flutter doctor -v`, `flutter build apk --debug`, `flutter run`, adb warning, `ADB_REJECT_KILL_SERVER=1 adb kill-server`), and record results.

## Slice 5: Documentation & Reconciliation (Plan step 5)
- [ ] 6.5.1 Reconcile `docs/architecture_design.md` and `docs/product_specification.md` (remove pending Phase 6 marks).
- [ ] 6.5.2 Update `README.md` with `--with` usage, Flutter toolchain details, and host adb server instructions.
- [ ] 6.5.3 Verify all tests and clippy pass, record outcome in `vdesign/plan.md`, and update `vdesign/checkpoint.md`.
