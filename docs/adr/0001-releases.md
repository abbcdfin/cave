# ADR 0001 — Releases: version scheme, channels and fixes

- **Status:** Accepted, 2026-11-25. Consolidates the versioning decisions of 2026-11-10
  (Phase 7) and 2026-11-19 (`v0.3.0`), and adopts the vibe framework's branching model.
- **Mechanisms:** [architecture_design.md › Packaging & Distribution](../architecture_design.md#packaging--distribution).

## Context

cave is used mainly by its owner, who installs it as the Arch `cave-git` package built by
`packaging/arch/PKGBUILD`. A VCS package takes its version from `git describe`, so tags
anchor the count; before Phase 7 nothing was tagged and builds were `0.1.0.rN`. The
package is not yet on the AUR. Until now, every commit went to `main`, so the package
installed work in progress.

## Decision

**Version scheme.** Semantic Versioning, with one release number, in `Cargo.toml`. A
release is the commit that sets `version = "X.Y.Z"` and adds its `CHANGELOG.md` entry,
tagged `vX.Y.Z` (annotated, message `vX.Y.Z — <summary>`); a pushed tag never moves. A
release follows each increment that changes what a user sees (an option, a profile, a
mount), as a minor bump while the version is `0.x`; fixes and documentation ship with the
next release.

**What counts as breaking.** A change after which an existing invocation, a user
`profiles.toml` or a session's mounts behave differently with no action by the user:
removing or renaming an option, changing the written text of a built-in entry (user
overrides match by text), narrowing a built-in mount, turning a warning fatal. While the
version is `0.x`, a breaking change is a minor bump whose `CHANGELOG.md` entry carries an
**Upgrade** line; from `1.0.0` it is a major bump. A change to what the sandbox exposes is
listed in the entry whichever way it goes.

**Channels and the push that publishes each.**

| Channel | Builds | Published by |
|---|---|---|
| `cave-git` from the repository's `PKGBUILD` | GitHub `main` | pushing `main` with its tag |
| `cave-git` on the AUR (pending) | GitHub `main` | the same push; the AUR repository is pushed only when the `PKGBUILD` changes |
| Source | a `vX.Y.Z` tag | pushing the tag |

`main` moves only to a tagged release, by fast-forward from `dev`, and `main`, `dev` and the
tag are pushed together, so every channel installs only releases (`X.Y.Z.r0.gHASH`). Work
lands on `dev`; running unreleased work means a local `cargo build`. `pkgver()` derives the
version and has no hard-coded fallback, so a clone without a reachable tag fails the
build. The `pkgver=` line `makepkg` rewrites in the committed `PKGBUILD` is discarded after
each build and updated only when publishing to the AUR.

**Fixes for older lines.** Only the latest release is supported. A fix lands on `dev` and
ships with the next release; one that must ship before unreleased work is ready goes on
`fix/<slug>`, branched from the latest release tag, and is released as a patch: `main`
fast-forwards to the patch's tag, which then merges into `dev`.

## Rejected alternatives

- **A hard-coded `pkgver()` fallback:** a clone without a tag would invent a version.
- **First tag `v0.1.0`:** the count restarts at a tag, and `0.1.0.r1` sorts below the
  installed `0.1.0.r35`, which pacman treats as a downgrade. `v0.2.0` was the first tag.
- **Committing `makepkg`'s `pkgver=` rewrite:** the commit becomes the one the next build
  counts, so the line can never be current.
- **`main` carrying work in progress:** `cave-git` would install unreleased commits, and a
  version would not name a release.
- **A `-dev` version on `dev`:** an updater comparing versions would receive it.
- **A static `cave` release package now:** deferred while `cave-git` is the only channel
  in use; [Packaging & Distribution](../architecture_design.md#packaging--distribution)
  keeps it as a later companion.

## Consequences

- The owner's own `cave-git` upgrades only at a release; dogfooding unreleased work needs a
  local build installed by hand (README › From Source).
- Every user-visible increment costs a release: a version bump, a changelog entry, a tag
  and a push the user approves.
- `CHANGELOG.md` is the one place a user learns what to change on upgrade.
