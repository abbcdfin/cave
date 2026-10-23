# Checkpoint

## Current State
- Phase 5 (SSH via the host ssh-agent) complete and collapsed into `vdesign/plan.md`:
  - Host `ssh-agent` socket forwarded RO to `/run/user/$UID/ssh-agent.sock` (`0700` runtime directory permissions), `.ssh/config` and `.ssh/known_hosts` mounted RO, private keys (`~/.ssh/id_*`) hidden.
  - `/etc/ssh/ssh_config.d` masked with a private `tmpfs` (`defaults.tmpfs` skips missing host paths via `p.exists()`), resolving OpenSSH `nobody`-owned drop-in rejection in rootless user namespaces.
  - Single `policy::resolve_ssh_socket` used by `main.rs` and `--doctor`; `no_ssh` handled in `main.rs` only; `build_mount_plan` reads no config and no environment (it stats and canonicalizes paths on disk).
  - `--doctor` verifies both `ssh -G github.com` and `ssh-add -l` inside a test sandbox.
  - Live `ssh -T git@github.com`, `git ls-remote`, and scratch branch push/delete over SSH verified and confirmed.
  - 25 unit tests and 20 boundary tests passing (45 total), 0 clippy warnings.
- Phases 1–5 complete.

## Next Steps
- Optional host housekeeping: move `ServerAliveInterval 120` from `/etc/ssh/ssh_config.d/50-general-client.conf` into `~/.ssh/config` if desired inside `cave`.
- Publish `cave-git` to the AUR.
- Evaluate candidates in `vdesign/plan.md` when ready for Phase 6.
