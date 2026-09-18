# Checkpoint

## Current State
- Phase 1 (Backend Spike & Decision) complete and reviewed 2026-09-18; collapsed into [`plan.md`](plan.md).
  - Decisions in [`docs/architecture_design.md`](../docs/architecture_design.md): `bwrap` backend, direct terminal inheritance, Rust CLI, loose coupling to the `bwrap` binary, `--dir /tmp` host tmp reuse. Startup overhead reproduced at 9.16 ms.
  - Review fixes applied to [`vdesign/utils/spike-bwrap.sh`](utils/spike-bwrap.sh): `~/.rustup` (cargo was unusable inside the sandbox), `/var`, `/var/tmp`, `/sys`, `/opt`, and a `/etc/resolv.conf` bind that keeps `/run` private on systemd-resolved hosts. **Not yet re-run** — verify before relying on it.
  - Path classification moved out of the collapsed active plan into the architecture doc.
- No code yet.

## Next Steps
- Re-run the spike to confirm the added mounts: `cargo --version`, `claude --version`, name resolution, `/var/tmp` writable, `/var` read-only.
- Start Phase 2 (Walking Skeleton: `cave claude` in Rust, boundary tests first), settling the three items reopened by the review in `plan.md`.
