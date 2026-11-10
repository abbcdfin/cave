# Checkpoint

## Current State
- Phase 6 (Toolchains, `cave --with flutter`) complete and collapsed into `vdesign/plan.md`:
  - Added `[toolchains.NAME]` configuration table beside `profiles` with `requires`, `rw_cache`, `runtime`, and `check_tcp` fields (`ToolchainConfig`, `UserToolchainConfig`, `TcpCheckConfig`). Layered in `merge_user_config` with append and move-between-lists rules.
  - Repeatable `--with NAME` flag in `cli::parse_args` accepted for both execution and `--doctor`; unknown toolchain names exit 1.
  - Toolchain mounts rendered after agent profile and before working directory in `--with` argument order, with `rw_cache` mounts preceding `runtime` mounts. `build_mount_plan` inspects candidate paths on disk and warns if `--dir` or working directory overrides root/home.
  - Unified toolchain entry resolution and classification in `policy::resolve_entry` and `policy::resolve_toolchain` over `ExpansionLookup` trait (`SystemLookup`), canonicalizing binaries via `bwrap::which_binary` and normalizing `..` lexically without host `/usr/bin` leakage.
  - Engine boundary protection: toolchain entries resolving to `/`, `$HOME`, or an ancestor containing `$HOME` exit 1 with a clear error without issuing `--dir` warnings. Unresolved `requires` entries exit 1 naming the missing requirement. Unresolvable or missing non-requires entries warn to stderr and are skipped.
  - Startup TCP connectivity diagnostics via `policy::check_tcp_port` (`TcpStream::connect_timeout`). Total startup overhead with `--with flutter` measured at 10.79 ms avg (min 9.65 ms); toolchain resolution adds ~0.75 ms vs base execution (< 100 ms budget).
  - Built-in `[toolchains.flutter]` in `src/profiles.toml` with `requires = ["${bin:flutter}", "${bin:flutter}/../../packages/flutter_tools"]`.
  - Verified live on host:
    - `cave --with flutter sh -c 'flutter doctor -v'` succeeded with exit code 0 finding all SDK and toolchain components with zero write errors on read-only paths.
    - `cave --with flutter sh -c 'flutter build apk --debug'` succeeded in 14.3s, reusing Gradle caches from host build (38.9s).
    - `cave --with flutter flutter run -d cbe5535f` connected to OnePlus 8T (`KB2000`), installed and started the app, and hot-reloaded modified `lib/main.dart` across the sandbox in 438ms.
    - Host adb server guard refusal verified: `ADB_REJECT_KILL_SERVER=1 adb start-server` on host caused `adb kill-server` inside cave to be rejected (`error: kill-server rejected by remote server`), leaving host daemon alive.
    - Secrets isolation verified: `~/.gradle/gradle.properties` and `~/.android/adbkey` are hidden (`No such file or directory`).
    - `cave --doctor --with flutter` verified.
  - 36 unit tests and 21 boundary tests passing (57 total), 0 clippy warnings (`-D warnings`). Separate mutation checks on 1a (Case 5) and 1b (Case 4) verified.
  - Documentation, product spec, and README reconciled.
- Phases 1–6 complete.

## Next Steps
- Publish `cave-git` to the AUR or tag a release.
- Review unnumbered candidates in `vdesign/plan.md` for next increment acceptance.
