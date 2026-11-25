# Development Environment

- **Rust** 1.94+ (`cargo`, `rustc`), edition 2024.
- **Bubblewrap** (`/usr/bin/bwrap`) with unprivileged user namespaces: the boundary tests
  in `tests/boundary_tests.rs` start real sandboxes.
- **Checks:** `cargo test` (unit tests in `src/`, boundary tests in `tests/`) and
  `cargo clippy --all-targets -- -D warnings`. `vdesign/utils/check_build_fresh.sh`
  checks that a repeat `cargo build` is fresh (`build.rs` rerun triggers).
- **Packaging:** `makepkg` in `packaging/arch/` builds `main` from GitHub, not the working
  tree; a local build installs as the README's "From Source" describes.
- **Agent sessions run inside cave** (`mctl.toml`: `cave --dir <mctl checkout> claude`,
  `cave agy`, `cave codex`). Their `$HOME` is cave's private tmpfs, so the host's
  `~/.config/cave/profiles.toml` is hidden and `cave --doctor` reports the sandbox's view.
  The boundary tests pass nested. Checks that need the host — a connected device,
  `pacman -U`, host-side session resume — need the user.
