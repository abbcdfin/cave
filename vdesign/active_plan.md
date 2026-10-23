# Active Plan: Phase 4 Remediation — Arch Linux Packaging & Distribution

## Context & Objectives
Reopened per review findings:
1. `makepkg` validation was premature: package was built before `LICENSE` and `examples/` were committed and pushed to GitHub; resulting package was missing both files.
2. README installation claimed `makepkg -si` installs local changes; it installs the pushed git branch.
3. README claimed `~/.config/cave/` is automatically hidden; it is only hidden because `$HOME` starts empty without `.config` mounted.
4. README example included `.npmrc` under `ro_home`, contradicting the architecture decision (decision 2026-10-22 keeps `.npmrc` out).
5. Architecture docs lacked detailed rationale for a `-git` package versus a tagged release.
6. PKGBUILD had dead `elif` for `LICENSE.md`, compiled all targets in release `build()`, and ran boundary tests in `check()` which fail in clean chroots lacking user namespaces.

## Tasks

- [x] **4.R1: Documentation fixes in `README.md`**
  - Clarify that `makepkg -si` builds the pushed GitHub branch, while `cargo build --release` tests local uncommitted changes.
  - Clarify that `~/.config/cave/` is hidden by default unless a parent directory is explicitly mounted.
  - Replace `.npmrc` in `ro_home` example with a harmless path (`.config/git`).
- [x] **4.R2: Architecture design documentation refinement**
  - Record detailed rationale for Arch `-git` package vs release tarball in `docs/architecture_design.md`.
- [x] **4.R3: PKGBUILD refinement**
  - Change `build()` to `cargo build --frozen --release --bin cave` (avoid compiling test harnesses in release).
  - Change `check()` to `cargo test --frozen --lib --bins` (avoid boundary tests that fail in chroot environments lacking userns).
  - Remove dead `elif LICENSE.md` branch in `package()`.
- [ ] **4.R4: Commit and push to GitHub**
  - Stage and commit all uncommitted files.
  - Push commit to `origin/main` on GitHub so remote source contains `LICENSE` and `examples/`.
- [ ] **4.R5: Rebuild and verify package archive**
  - Run `makepkg -f` in `packaging/arch/`.
  - Inspect archive contents using `tar -tf` to verify `usr/bin/cave`, `usr/share/doc/cave/README.md`, `usr/share/doc/cave/profiles.toml.example`, and `usr/share/licenses/cave/LICENSE` are all physically present.
- [ ] **4.R6: Verification and collapse**
  - Verify `cargo test` and `cargo clippy`.
  - Collapse Phase 4 in `plan.md`, `active_plan.md`, and `checkpoint.md`.

## Exit Criteria
- `tar -tf packaging/arch/cave-git-*.pkg.tar.zst` shows `cave`, `README.md`, `profiles.toml.example`, and `LICENSE`.
- `README.md` reflects true installation semantics, accurate sandbox inheritance notes, and harmless config examples.
- `docs/architecture_design.md` records complete packaging rationale.
- All tests pass, clippy clean.



