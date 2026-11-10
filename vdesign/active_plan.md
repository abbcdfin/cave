# Active Plan — Phase 7: Versioning (`cave --version` names its build)

Status: BUILDING. Exit criteria: see `vdesign/plan.md` › Phase 7 ("Done when").
Decisions & rationale: `docs/architecture_design.md` › Packaging & Distribution › Versioning.

## Slice 1: Pure Version Formatting & Unit Tests (Plan step 1a)
- [x] 7.1.1 Implement pure version formatting function `format_version(pkg_version: &str, git_describe: Option<&str>) -> String` in `src/cli.rs`.
- [x] 7.1.2 Update `print_version()` in `src/cli.rs` to use `format_version` with `CARGO_PKG_VERSION` and `option_env!("CAVE_GIT_DESCRIBE")`.
- [x] 7.1.3 Unit tests in `src/cli.rs` for version output permutations: with tag describe, hash-only describe, dirty suffix, and absent git describe; plus CLI test that `cave --version` output begins with `cave <CARGO_PKG_VERSION>`.

## Slice 2: Git Build Script & Rebuild Triggers (Plan step 1b)
- [x] 7.2.1 Create `build.rs` to run `git -C $CARGO_MANIFEST_DIR describe --long --tags --dirty --always` only when toplevel matches `CARGO_MANIFEST_DIR`, and pass `cargo:rustc-env=CAVE_GIT_DESCRIBE=...` without failing on git errors.
- [x] 7.2.2 Add complete rebuild triggers in `build.rs`: `src/`, `Cargo.toml`, `build.rs`, and git tracking paths (`HEAD`, `index`, `packed-refs`, `refs/tags`, branch ref via `git rev-parse --git-path`).
- [x] 7.2.3 Verify rebuild behavior on commit / file edit without `cargo clean`, and verify graceful fallback to plain package version when `.git` is absent.

## Slice 3: Release 0.2.0 Bump & PKGBUILD Simplification (Plan step 2)
- [x] 7.3.1 In a single commit: update `Cargo.toml` to `version = "0.2.0"`, update `Cargo.lock`, and simplify `pkgver()` in `packaging/arch/PKGBUILD` to strictly use `git describe --long --tags | sed ...` with no hardcoded fallback.
- [x] 7.3.2 Verify test that `pkgver()` fails non-zero on a clone with no reachable tag.

## Slice 4: Tag, Push & Package Build Verification (Plan step 3)
- [x] 7.4.1 Create annotated tag `v0.2.0 -m "cave 0.2.0"` on the release commit.
- [x] 7.4.2 Ask user for approval to push `main` and `v0.2.0` to GitHub (`origin`).
- [x] 7.4.3 Upon user confirmation, push `main` and `v0.2.0`.
- [x] 7.4.4 Run `makepkg` in `packaging/arch/`. Verify package name is `cave-git-0.2.0.r…`, binary `--version` matches commit and has no `-dirty` suffix. Discard `PKGBUILD` pkgver rewrite via `git checkout packaging/arch/PKGBUILD`.
- [ ] 7.4.5 User installs with `sudo pacman -U …` and confirms no downgrade warning and matching commit.

## Slice 5: Documentation & Reconciliation (Plan step 4)
- [x] 7.5.1 Remove "pending Phase 7" marks in `docs/architecture_design.md` and `docs/product_specification.md`.
- [x] 7.5.2 Update `README.md` with version output explanation, release workflow steps, and discarding `makepkg` pkgver rewrite.
- [ ] 7.5.3 All tests and clippy pass; record outcome in `vdesign/plan.md` and update `vdesign/checkpoint.md`.
