# Checkpoint

## Current State
- Phase 3 (Agent Profiles & Diagnostics) completed and collapsed into `plan.md`.
  - Added profiles for `agy` (`~/.gemini` RW state), `aider` (`~/.aider` RW state, `~/.aider.conf.yml` RO runtime), and shells (`sh`, `bash`, `zsh`).
  - Swapped profile mount loop in `policy.rs` so `state` renders before `runtime`, enabling read-only subpath protection inside read-write directories.
  - Implemented unknown-agent warning on stderr when no profile matches the agent basename, silencing known shells.
  - User config file layering (`${XDG_CONFIG_HOME:-$HOME/.config}/cave/profiles.toml`) over embedded defaults with append semantics, downgrade-by-re-declaration, and fatal located parse errors.
  - Config loaded once (`main.rs`, `cli.rs`) and passed as `&ConfigData`: neither `build_mount_plan` nor `prepare_host_state` reads config, and `prepare_host_state` is the only host write.
  - Completed `cave --doctor` diagnostics: verifies `bwrap` availability, unprivileged user namespaces, AppArmor restrictions, `/var/run` symlink to `/run`, sandboxed `/etc/resolv.conf` nameserver presence, non-fatal external DNS lookup, and prints config sources and effective merged profile table.
  - Dogfooding and manual verification: live interactive session persistence confirmed across host/cave boundary for `claude` and `agy`.
  - Test suite passing: 17 unit tests, 19 boundary tests (36 total, 0 clippy warnings).
  - Measured release startup overhead: 8.96 ms (budget < 100 ms).
- All planned roadmap phases in `vdesign/plan.md` (Phases 1-3) are complete.

## Next Steps
- Review and dogfood `cave` across supported agents (`claude`, `agy`, `aider`).
- Evaluate candidate backlog in `vdesign/plan.md` for potential next phase (e.g. generic CLI fallback profile, network policy filtering, ephemeral git branch / copy-on-write snapshotting, opt-in Docker access).
