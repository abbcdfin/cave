# Checkpoint

## Current State
- Phase 6 (Toolchains, `cave --with flutter`) complete and collapsed into `vdesign/plan.md`:
  - Added `[toolchains.NAME]` configuration table beside `profiles` with `requires`, `rw_cache`, `runtime`, and `check_tcp` fields (`ToolchainConfig`, `UserToolchainConfig`, `TcpCheckConfig`). Layered in `merge_user_config` with append and move-between-lists rules.
  - Repeatable `--with NAME` flag in `cli::parse_args` accepted for both execution and `--doctor`; unknown toolchain names exit 1.
  - Mount order renders toolchains after the agent profile and before the working directory in `--with` argument order, with `rw_cache` mounts preceding `runtime` mounts. Pure function `build_mount_plan` creates no host paths.
  - Pure entry expansion `policy::expand_entry` / `policy::expand_string` over `Lookup` trait (`SystemLookup`), canonicalizing binaries via `bwrap::which_binary` and normalizing `..` lexically. Unresolved `requires` entries exit 1 naming the toolchain and missing entry; other unresolved entries warn to stderr and are skipped. Resolved paths pass through `cli::check_warning`.
  - Non-blocking TCP connectivity diagnostics via `policy::check_tcp_port` (`TcpStream::connect_timeout`). Measured startup overhead with `--with flutter` is 0.75 ms (budget < 100 ms).
  - Built-in `[toolchains.flutter]` in `src/profiles.toml`. Verified secrets isolation (`~/.gradle/gradle.properties` and `~/.android/adbkey` hidden), host adb server guard refusal (`ADB_REJECT_KILL_SERVER=1` rejects inside `adb kill-server`), missing server warnings, and doctor diagnostics.
  - 35 unit tests and 21 boundary tests passing (56 total), 0 clippy warnings. Mutation check on `requires` passed.
  - Documentation and README reconciled, pending Phase 6 marks removed.
- Phases 1–6 complete.

## Next Steps
- Connect a physical phone / emulator and test `cave --with flutter claude` / `flutter run` with hot reload if desired.
- Publish `cave-git` to the AUR.
- Review unnumbered candidates in `vdesign/plan.md` for next increment acceptance.
