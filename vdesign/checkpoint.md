# Checkpoint

## Current State
- Phase 1 (Backend Spike & Decision) complete and reviewed 2026-09-18; collapsed into [`plan.md`](plan.md).
  - Decisions in [`docs/architecture_design.md`](../docs/architecture_design.md): `bwrap` backend, direct terminal inheritance, Rust CLI, loose coupling to the `bwrap` binary, `--dir /tmp` host tmp reuse. Startup overhead reproduced at 9.16 ms.
  - Review fixes applied to [`vdesign/utils/spike-bwrap.sh`](utils/spike-bwrap.sh): `~/.rustup` (cargo was unusable inside the sandbox), `/var`, `/var/tmp`, `/sys`, `/opt`, and a `/etc/resolv.conf` bind that keeps `/run` private on systemd-resolved hosts. **Not yet re-run** — verify before relying on it.
  - Path classification moved out of the collapsed active plan into the architecture doc.
- No code yet.

- Phase 2 planned 2026-09-18: the four open policy decisions (environment, mount-order override rule, `~/.cache` / `~/.config`, agent profile selection) are recorded in `docs/architecture_design.md`; the board is in [`active_plan.md`](active_plan.md).

## Next Steps
- Phase 2 task 2.0: re-run the spike and test `claude` interactively (resize, Ctrl-C, Ctrl-Z, `claude -c`) before writing code.
