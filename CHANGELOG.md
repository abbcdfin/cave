# Changelog

What each cave release changes for its users, newest first. Versions follow
[Semantic Versioning](https://semver.org/) in its 0.x form; when to release is decided in
[ADR 0001](docs/adr/0001-releases.md). An **Upgrade** line
names any step a release needs.

## v0.3.0 — 2026-11-19

- `--profile NAME` adds agent profiles beside the command's own, so several agents can
  share one session (`cave --profile claude,codex,agy tmux`), or a wrapped agent gets its
  profile (`cave --profile claude npx @anthropic-ai/claude-code`). Repeatable and
  comma-separated; an unknown or empty name exits 1; each profile mounts once.
- `--with` takes comma-separated names too.
- Built-in `codex` profile (`${CODEX_HOME:-.codex}`).
- The `claude` profile follows the agent's own relocation: `${CLAUDE_CONFIG_DIR:-.claude}`
  for state and `${bin:claude}/..` for the installed versions.
- Profile entries take the toolchain syntax (`${VAR}`, `${VAR:-default}`, `${bin:NAME}`,
  host-absolute paths). An entry resolving to `/`, `$HOME` or a folder containing
  `$HOME` exits 1 before anything is created on the host.
- `cave --doctor --profile NAME` shows each entry and what it resolved to.
- **Upgrade:** a user `profiles.toml` matches built-in entries by their written text. An
  override of claude's `.claude` or `.local/share/claude` must now name
  `${CLAUDE_CONFIG_DIR:-.claude}` or `${bin:claude}/..`.

## v0.2.0 — 2026-11-10

The first tagged release; earlier `cave-git` builds were `0.1.0.rN`.

- Sandboxes an agent command with `bwrap` at host paths, keeping the host terminal:
  read-write working directory and agent state, read-only host runtime, private `$HOME`,
  `/tmp` and `/run`, hidden credentials and host session services.
- Built-in profiles for `claude`, `agy`, `aider` and the shells `sh`, `bash`, `zsh`; an
  unknown agent gets an ephemeral `$HOME` with a warning.
- `--dir` and `--ro-dir` add allowed directories; a user file
  (`~/.config/cave/profiles.toml`) layers over the built-in table.
- The host ssh-agent is forwarded while private keys stay hidden; `--no-ssh` turns it off.
- `--with NAME` adds a toolchain; built-in `flutter` uses the Android SDK and the host's
  adb server.
- `cave --doctor` checks the backend, DNS, SSH and the merged configuration.
- `cave --version` names the release and its commit (`cave 0.2.0 (v0.2.0-0-ge640705)`).
