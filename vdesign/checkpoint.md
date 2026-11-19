# Checkpoint

## Current State
- Phase 6 (Toolchains, `cave --with flutter`) complete and collapsed into `vdesign/plan.md`:
  - Added `[toolchains.NAME]` configuration table beside `profiles` with `requires`, `rw_cache`, `runtime`, and `check_tcp` fields (`ToolchainConfig`, `UserToolchainConfig`, `TcpCheckConfig`). Layered in `merge_user_config` with append and move-between-lists rules.
  - Repeatable `--with NAME` flag in `cli::parse_args` accepted for both execution and `--doctor`; unknown toolchain names exit 1.
  - Toolchain mounts rendered after agent profile and before working directory in `--with` argument order, with `rw_cache` mounts preceding `runtime` mounts. `build_mount_plan` inspects candidate paths on disk and warns when the working directory is `/` or `$HOME`; `cli::check_warning` warns for such `--dir` / `--ro-dir` paths in `parse_args`.
  - Unified toolchain entry resolution and classification in `policy::resolve_entry` and `policy::resolve_toolchain` over `ExpansionLookup` trait (`SystemLookup`), canonicalizing binaries found by `bwrap::which_binary` (which falls back to `/usr/bin` only when `PATH` is unset) and normalizing `..` lexically.
  - Engine boundary protection: toolchain entries resolving to `/`, `$HOME`, or an ancestor containing `$HOME`, as written or through a symlink, exit 1 with a clear error without issuing `--dir` warnings. Unresolved `requires` entries exit 1 naming the missing requirement. Unresolvable or missing non-requires entries warn to stderr and are skipped.
  - Startup TCP connectivity diagnostics via `policy::check_tcp_port` (`TcpStream::connect_timeout`). Total startup overhead with `--with flutter` measured at 10.79 ms avg (min 9.65 ms); toolchain resolution adds ~0.75 ms vs base execution (< 100 ms budget).
  - Built-in `[toolchains.flutter]` in `src/profiles.toml` with `requires = ["${bin:flutter}", "${bin:flutter}/../../packages/flutter_tools"]`.
  - Verified live on host:
    - `cave --with flutter sh -c 'flutter doctor -v'` succeeded with exit code 0 finding all SDK and toolchain components with zero write errors on read-only paths.
    - `cave --with flutter sh -c 'flutter build apk --debug'` succeeded in 14.3s, reusing Gradle caches from host build (38.9s).
    - `cave --with flutter flutter run -d cbe5535f` connected to OnePlus 8T (`KB2000`), installed and started the app, and hot-reloaded modified `lib/main.dart` across the sandbox in 438ms.
    - Host adb server guard refusal verified: `ADB_REJECT_KILL_SERVER=1 adb start-server` on host caused `adb kill-server` inside cave to be rejected (`error: kill-server rejected by remote server`), leaving host daemon alive.
    - Secrets isolation verified: `~/.gradle/gradle.properties` and `~/.android/adbkey` are hidden (`No such file or directory`).
    - `cave --doctor --with flutter` verified.
  - 37 unit tests and 21 boundary tests passing (58 total), 0 clippy warnings (`-D warnings`). Separate mutation checks on 1a (Case 5), 1b (Case 4), and the symlink case verified.
  - Documentation, product spec, and README reconciled.
- Phase 7 (Versioning, `cave --version` names its build) complete and collapsed into `vdesign/plan.md`:
  - Added `build.rs` to embed `git describe --long --tags --dirty --always` via `CAVE_GIT_DESCRIBE` only when `git rev-parse --show-toplevel` matches `CARGO_MANIFEST_DIR`, never failing if git or metadata is absent. Watched `src`, `Cargo.toml`, `build.rs`, and git ref paths (`HEAD`, `index`, `packed-refs`, `refs/tags`, branch ref).
  - Added `cli::format_version` (`cave <version> (<describe>)` or plain `cave <version>`) used by `cli::print_version`.
  - Released `v0.2.0`: bumped `Cargo.toml` and `Cargo.lock` to `0.2.0`, removed hardcoded fallback from `pkgver()` in `packaging/arch/PKGBUILD` (`makepkg` fails non-zero on tagless clones), tagged `v0.2.0` on commit `e640705`, pushed `main` and `v0.2.0` to GitHub, and built `cave-git-0.2.0.r0.ge640705-1-x86_64.pkg.tar.zst` (discarding local `pkgver=` rewrite).
  - Verified host `pacman -U` upgrade from `0.1.0.r35.gac678da-1` with no downgrade warning; `pacman -Q cave-git` (`0.2.0.r0.ge640705-1`) and `cave --version` (`cave 0.2.0 (v0.2.0-0-ge640705)`) match.
  - 38 unit tests and 22 boundary tests passing (60 total), 0 clippy warnings (`-D warnings`).
  - Reconciled `docs/architecture_design.md`, `docs/product_specification.md`, and `README.md`.
- Phases 1–7 complete.

## Next Steps
- Publish `cave-git` to the AUR.
- Review unnumbered candidates in `vdesign/plan.md` (including `--profile NAME` multi-agent sessions) for next increment acceptance.
