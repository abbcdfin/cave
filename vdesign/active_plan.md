# Active Plan — Phase 5: SSH via the host ssh-agent

Status: ACCEPTED 2026-10-23. Decisions and rationale: `docs/architecture_design.md` › SSH.

Build thin, then widen: 5.1 alone makes `ssh-add -l` work inside cave; each later item widens it.

- [ ] 5.1 Forward the agent socket (thin path)
  - `main.rs` reads `SSH_AUTH_SOCK` and passes `Option<&Path>` into `build_mount_plan`; policy reads no environment.
  - When the socket exists: `BindReadOnly` from it to `/run/user/$UID/ssh-agent.sock`, rendered after the `/run` tmpfs, and the private runtime dir keeps `0700`; `SSH_AUTH_SOCK` goes to `set_env` and out of `unset_env`. Otherwise `SSH_AUTH_SOCK` stays in `unset_env`.
  - Confirm `connect()` works through a read-only bind; if it does not, use `BindReadWrite` and record why in docs.
  - Exit: `cave sh -c 'ssh-add -l'` prints the host key fingerprint; `cave sh -c 'ls ~/.ssh'` shows no private key.
- [ ] 5.2 `~/.ssh/config` and `~/.ssh/known_hosts` as `ro_home` defaults in `src/profiles.toml` (data, no code).
  - Exit: `cave sh -c 'ssh -T git@github.com'` authenticates; `cave sh -c 'touch ~/.ssh/config'` fails.
- [ ] 5.3 `--no-ssh` flag: `parse_args`, a `RunOptions` field, `--help`, README CLI table. After the agent command it passes through to the agent like every other argument.
  - Exit: `cave --no-ssh sh -c 'ssh-add -l'` exits 2 (no agent).
- [ ] 5.4 Stale socket: `SSH_AUTH_SOCK` set, socket missing → `Warning: SSH_AUTH_SOCK points to a missing socket; ssh-agent not forwarded.` on stderr, session continues. Unset → silent. Warning lives in `main.rs`, beside the unknown-agent warning.
- [ ] 5.5 `--doctor`: run `ssh-add -l` inside the sandbox. Exit 0 → `[OK]` with key count; 1 → `[WARN]` agent forwarded, no keys loaded; 2 or `SSH_AUTH_SOCK` unset → `[WARN]` not available; stale socket → `[WARN]`; `ssh-add` absent → `[WARN]` check skipped. None of these is `[FAIL]`: SSH is optional.
- [ ] 5.6 Tests
  - Unit: socket given → RO bind at the fixed target, after the `/run` tmpfs, `SSH_AUTH_SOCK` in `set_env` and not in `unset_env`; no socket → no bind, `SSH_AUTH_SOCK` unset. `parse_args` with `--no-ssh` before and after the agent command.
  - Boundary: start a throwaway `ssh-agent -a <tmp>/agent.sock` with a generated key; inside cave `ssh-add -l` shows its fingerprint; the key file in the test home is invisible; `~/.ssh/config` is read-only; `--no-ssh` and a stale path each make `ssh-add -l` exit 2, the stale path with the warning on stderr. Kill the agent at the end. Without `ssh-agent`/`ssh-keygen`, print a skip notice.
  - Mutation: drop the `SSH_AUTH_SOCK` `set_env` entry and confirm the boundary test fails.
- [ ] 5.7 Live under `cave claude` and `cave agy`: `git ls-remote origin` and a push to a scratch branch over ssh. Optionally `ssh-add -c` on the host and confirm the prompt appears for each signature.
- [ ] 5.8 README (`--no-ssh` in the CLI table, an SSH note in Configuration), then collapse into `plan.md`.

Phase exit: 5.1–5.8 checked, `cargo test` and `cargo clippy --all-targets` clean, live push from inside cave confirmed by the user.
