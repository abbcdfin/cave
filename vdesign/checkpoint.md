# Checkpoint

## Current State
- Phase 3 (Agent Profiles & Diagnostics) complete and collapsed into `plan.md`.
  - Added profiles for `agy` (`~/.gemini` RW state, `~/.gemini/antigravity-cli/bin` RO runtime), `aider` (`~/.aider` RW dir and config/history files), and shells (`sh`, `bash`, `zsh`).
  - Swapped profile mount loop in `policy.rs` so `state` renders before `runtime`, enabling read-only subpath protection inside read-write directories without schema changes.
  - Implemented unknown-agent warning on stderr when no profile matches the agent basename, silencing known shells.
  - Implemented user config file layering (`${XDG_CONFIG_HOME:-$HOME/.config}/cave/profiles.toml`) over embedded defaults with append semantics, downgrade-by-re-declaration, and fatal located parse errors.
  - Completed `cave --doctor` diagnostics: verifies `bwrap` availability, unprivileged user namespace permissions, AppArmor restrictions, `/var/run` symlink to `/run`, sandbox DNS resolution (`ahostsv4 localhost`), and prints config sources and effective merged profile table.
  - Test suite passing: 16 unit tests, 18 boundary tests (34 total, 0 clippy warnings).
  - Measured release startup overhead: 8.96 ms (budget < 100 ms).
- All planned roadmap phases in `vdesign/plan.md` (Phases 1-3) are complete.

## Next Steps
- Review and dogfood `cave` with supported agents (`claude`, `agy`, `aider`).
- Evaluate candidate backlog in `vdesign/plan.md` for potential next phase (e.g. network policy filtering, ephemeral git branch/COW snapshotting, opt-in Docker access).
