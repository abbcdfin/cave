# Active Plan: Phase 3 — Agent Profiles & Diagnostics

## Context & Objectives
Widen cave from one agent to several. Sequenced for dogfooding: `agy` support first, so
cave's own development runs under cave; then the user config file, so later profiles can be
validated by editing a file instead of rebuilding; then profile refinement and diagnostics.

## Investigation outcome (2026-10-22)
Resolves the Phase 3 precondition "confirm `agy` is a terminal agent rather than a GUI launcher":

- **`agy` is a terminal CLI.** `/usr/bin/agy` is an ELF binary offering `--continue`, `--print`,
  `--prompt-interactive`, `--mode`, and `--dangerously-skip-permissions`. `/usr/bin/antigravity`
  is a shell script that launches the GUI IDE and is out of scope.
- **The CLI's state is `~/.gemini/`**, not the directories named after Antigravity:
  `antigravity-cli/` (`conversations` 136M, `brain` 220M, `history.jsonl`, `settings.json`,
  `conversation_summaries.db`, `annotations`, `knowledge`, `mcp`, `presence`, `installation_id`,
  `antigravity-oauth-token`, `bin` 17M, `log`, `cache`, `crashes`), `config/` (`config.json`,
  `mcp_config.json`, `projects`, `plugins`), and `GEMINI.md`.
- **The IDE's directories must not be declared.** `~/.antigravity` (848M, `extensions`) and
  `~/.config/Antigravity` (391M Electron profile: `Cookies`, `GPUCache`, `Local Storage`)
  belong to the GUI, not to `agy`.

## Tasks

- [ ] **3.1: `agy` profile — thin slice**
  - Add `[profiles.agy]` declaring `state` as `~/.gemini` (`kind = "dir"`). No `runtime` entries
    yet; the read-only-subpath problem is 3.4.
  - Note: a read-write `~/.gemini` includes `antigravity-oauth-token`, so `agy` can read its own
    credentials — unavoidable for a running agent, and the reason `state` stays `~/.gemini`
    rather than anything wider (see `docs/architecture_design.md` › Inheritance model).
  - Boundary test: under `cave agy`, `~/.gemini` is read-write while `~/.config/Antigravity`,
    `~/.antigravity` and `~/.ssh` stay hidden.
  - Verify `cave agy --version`, then an interactive `cave agy` in this repo: a session started
    under cave resumes with `agy -c` on the host, and the reverse.
  - Exit: cave's own development runs under `cave agy --dangerously-skip-permissions`.
- [ ] **3.2: Unknown-agent warning**
  - Warn on stderr when no profile matches the basename: name the basename tried, state that
    `$HOME` is ephemeral so state will not persist, point at `--dir`. In `main.rs`;
    `build_mount_plan` stays pure.
  - Boundary test asserting the warning reaches stderr and the run still succeeds.
- [ ] **3.3: User config file layering**
  - Read `${XDG_CONFIG_HOME:-$HOME/.config}/cave/profiles.toml` when present; absent is not an error.
  - Append over the embedded table. Unit tests for append and for downgrade-by-re-declaration.
  - Parse failure exits non-zero naming file and line.
  - Boundary tests: a path added only by the user file is mounted, and `~/.config/cave/` is not
    visible inside the sandbox.
- [ ] **3.4: Profile refinement**
  - Decide how to express a read-only subpath inside a read-write state directory — `agy`'s
    `~/.gemini/antigravity-cli/bin` is the first case. Today `runtime` renders before `state`,
    so a read-only entry is covered by the later read-write mount. Options: render `state`
    before `runtime`, or replace the two lists with one ordered list per profile.
  - Add `aider` (`~/.aider.conf.yml` file and `~/.aider` directory, both present on this host)
    and a generic fallback, validated through the user config file before promotion into the
    embedded table.
- [ ] **3.5: `--doctor` completion**
  - Print the loaded config sources and the effective merged table.
  - Add the two checks deferred from Phase 2: resolve a name inside the sandbox, and confirm
    `/var/run` is a symlink to `/run`.

## Exit Criteria
- `claude`, `agy` and `aider` each run under cave with no engine change beyond 3.4's ordering decision.
- A new allowed path can be added by editing the user config file, with no rebuild.
- `cave --doctor` answers why a given path is or is not mounted.
