# cave

Run coding agents autonomously in **YOLO mode** on Linux without putting files outside your project at risk.

`cave` transparently sandboxes coding agents (such as Claude Code, Antigravity, Aider) in a restricted environment so you can run them with auto-approvals enabled, while preserving the exact interactive terminal experience as if running directly on your host.

## Why cave?

- **Frictionless Autonomy (YOLO Mode)**: Eliminates repetitive permission prompts without exposing your entire host system, credentials, or personal files to unintended agent actions.
- **Identical Terminal UX**: Transparently proxies PTY, interactive input, colors, keybindings, and window sizing.
- **Seamless Working Directory**: Automatically mounts and maps the current working directory, preserving file ownership and git context.

## Planned Workflow

Invoke `cave` followed by the agent CLI command and its arguments:

```bash
# Launch Claude Code in YOLO / auto-approve mode inside the cave sandbox
cave claude --dangerously-skip-permissions

# Launch other agents with their respective arguments
cave agy
cave aider --architect
```

The terminal opens directly into the agent session in your current working directory. cave passes agent arguments through unchanged: YOLO mode is on only when you pass the agent's own flag.

## Installation

### Arch Linux (AUR / PKGBUILD)

Build and install the package from the pushed GitHub branch:

```bash
cd packaging/arch
makepkg -si
```

Or install using an AUR helper once published:

```bash
yay -S cave-git
```

### From Source (Local Development)

To test uncommitted local changes directly:

```bash
cargo build --release
install -Dm755 target/release/cave ~/.local/bin/cave
```

### Versioning & Release Workflow

- **Build Metadata**: `cave --version` names the build:
  - Formatted as `cave <version> (<describe>)` (e.g. `cave 0.2.0 (v0.2.0-0-ge640705)`).
  - Uncommitted changes append `-dirty`.
  - Built from source without git metadata, it prints plain `cave <version>`.
- **Packaging (`PKGBUILD`)**: Derives `pkgver()` dynamically via `git describe --long --tags` (`0.2.0.rN.gHASH`).
- **Decisions**: version scheme, release channels, branches and fixes are in [ADR 0001](docs/adr/0001-releases.md); `main` holds releases only and work lands on `dev`.
- **Release Steps** (on `dev`):
  1. Bump version in `Cargo.toml`, update `Cargo.lock` (`cargo check`), and add the release's entry to [`CHANGELOG.md`](CHANGELOG.md).
  2. Commit the three files together: this is the release commit.
  3. Create annotated tag: `git tag -a vX.Y.Z -m "vX.Y.Z — <summary>"`.
  4. Fast-forward `main` and push it with `dev` and the tag: `git checkout main && git merge --ff-only dev && git push origin main dev vX.Y.Z`. The `pre-push` hook refuses `main` at an untagged commit.
  5. Run `makepkg` in `packaging/arch/`.
  6. Discard the `pkgver=` rewrite that `makepkg` leaves in `PKGBUILD`: `git checkout packaging/arch/PKGBUILD`.

## CLI Syntax

```bash
cave [cave-options] <agent-command> [agent-arguments...]
```

cave options must come before the agent command; everything from the agent command on is passed to the agent unchanged.

### Common Commands

| Invocation | Action |
| :--- | :--- |
| `cave claude --dangerously-skip-permissions` | Runs Claude Code in YOLO mode, sandboxed to the CWD |
| `cave <agent>` | Runs the specified agent command inside the isolated environment |
| `cave --profile <names> <cmd>` | Adds agent profiles (e.g. `claude,codex,agy`) beside the command (repeatable, comma-separated) |
| `cave --with <toolchain> <agent>` | Adds a project toolchain (e.g. `flutter`) to the session (repeatable, comma-separated) |
| `cave --no-ssh <agent>` | Disables forwarding the host SSH agent socket for this session |
| `cave --dir ../shared-lib claude` | Also allows read-write access to `../shared-lib` |
| `cave --ro-dir ~/reference claude` | Also allows read-only access to `~/reference` |
| `cave --version` | Prints version and build metadata |
| `cave --doctor` | Inspects sandbox backend availability and diagnostics (`--profile`, `--with`) |

## Configuration

`cave` works out of the box with embedded profiles for supported agents (`claude`, `codex`, `agy`, `aider`, `sh`, `bash`, `zsh`).

### Multi-Agent Sessions (`--profile NAME`)

Run multiple agents together (e.g. `cave --profile claude,codex,agy tmux` or `sh`) or launch wrapped agents (`cave --profile claude npx @anthropic-ai/claude-code`):

- **Additive & Ordered**: The command's own profile (if any) mounts first, followed by `--profile` names in order. `--profile` is repeatable and comma-separated (`--profile a,b` equals `--profile a --profile b`); each profile is mounted once. Unknown or empty names exit 1.
- **Relocation Support**: Built-in entries honor relocation variables (`${CLAUDE_CONFIG_DIR:-.claude}` and `${bin:claude}/..` for `claude`; `${CODEX_HOME:-.codex}` for `codex`). Values resolving to `/`, `$HOME`, or a folder containing `$HOME` exit 1 before anything is created on the host.
- **Cost**: Agents running in the same shared session can read and modify each other's logins and settings; under the clumsy-agent threat model, one agent can damage another's configuration. Run separate `cave` sessions on the same project when agents do not need to share a session.

### SSH Agent Forwarding

`cave` forwards the host's `ssh-agent` socket by default to enable git operations over SSH without disclosing private credentials:

- **Socket Forwarding**: Bound read-only at `/run/user/$UID/ssh-agent.sock` within a private runtime directory (`0700`).
- **Credential Isolation**: Private keys (`~/.ssh/id_*`) remain strictly hidden and are never mounted into the sandbox.
- **Config & Known Hosts**: `~/.ssh/config` and `~/.ssh/known_hosts` are mounted read-only by default when present on the host.
- **Host Drop-ins (`/etc/ssh/ssh_config.d`)**: System drop-in directories are masked with an empty tmpfs so OpenSSH does not reject host root-owned files (which appear as `nobody` inside unprivileged user namespaces). Host-wide custom SSH configurations belong in `~/.ssh/config`.
- **Opt-out**: Pass `--no-ssh` before the agent command (`cave --no-ssh <agent>`) to disable forwarding for that session.
- **Stale Sockets**: If `SSH_AUTH_SOCK` points to a missing or non-socket path, `cave` emits a warning to stderr and unsets the variable inside the sandbox without failing the session.

### Toolchain Support (`--with NAME`)

Toolchains add project-specific SDKs, caches, and configuration to an agent session without cluttering the global default sandbox:

- **Built-in `flutter`**:
  - Requires `${bin:flutter}` on `PATH` (resolves symlinks to locate the SDK root).
  - Mounts SDK root read-write (`rw_cache`), along with `~/.pub-cache`, `~/.gradle/caches`, `~/.gradle/wrapper`, and `~/.dartServer`.
  - Mounts Android SDK (`${ANDROID_HOME:-Android/Sdk}`), debug keystore (`~/.android/debug.keystore`), and `~/.config/flutter` read-only.
  - Sensitive files (`~/.gradle/gradle.properties`, `~/.android/adbkey`) stay hidden inside the private `$HOME` tmpfs.
  - Checks for host `adb` server at `127.0.0.1:5037` and warns with startup instructions if not running.
- **Host ADB Server Guard**:
  To prevent an agent from inadvertently killing the host `adb` server via `adb kill-server`, start the host server with:
  ```bash
  ADB_REJECT_KILL_SERVER=1 adb start-server
  ```
- **Custom Toolchains**:
  Define custom toolchains under `[toolchains.NAME]` in `~/.config/cave/profiles.toml`. Supports path expansion (`${VAR}`, `${VAR:-default}`, `${bin:NAME}`), `requires = [...]`, `rw_cache = [...]`, `runtime = [...]`, and `check_tcp = [{ addr, hint }]`.

To customize mount paths or define new agent profiles, create a configuration file at:
`~/.config/cave/profiles.toml` (or `${XDG_CONFIG_HOME:-$HOME/.config}/cave/profiles.toml`)

A starter template is available in [`examples/profiles.toml`](examples/profiles.toml) (installed to `/usr/share/doc/cave/profiles.toml.example`):

```bash
install -Dm644 /usr/share/doc/cave/profiles.toml.example ~/.config/cave/profiles.toml
```

### Example `~/.config/cave/profiles.toml`

```toml
[defaults]
# Additional system paths to mount read-only
ro_system = ["/opt/custom-tool"]

# Additional host home paths to mount read-only
ro_home = [".config/git", ".config/nvim"]

# Additional cache directories to mount read-write
rw_cache = [".cache/pip"]

# Define or extend an agent profile
[profiles.myagent]
runtime = [".config/myagent/config.json"]  # Read-only configuration
[[profiles.myagent.state]]
path = ".myagent"                          # Read-write state directory
kind = "dir"

# Override a built-in probed entry by matching its exact written text (not ".codex"):
# [profiles.codex]
# runtime = ["${CODEX_HOME:-.codex}"]
```

- **Layering**: User configuration appends to embedded defaults. Re-declaring an existing path moves it to the new class (e.g., downgrading `.cache` to `ro_home` makes it read-only). Matching compares entries by the text as written (e.g. `"${CODEX_HOME:-.codex}"`), not the resolved path.
- **Isolation**: `~/.config/cave/` is hidden by default (since `$HOME` is an empty private tmpfs without `.config` mounted). It is visible only if you explicitly mount a parent directory (e.g. `--dir ~/.config`).
- **Diagnostics**: Run `cave --doctor` to inspect loaded configuration sources and the effective merged table.

## Architecture & Roadmap

- **Product Specification (What & Why)**: [`docs/product_specification.md`](docs/product_specification.md)
- **Architecture Design (How)**: [`docs/architecture_design.md`](docs/architecture_design.md)
- **Releases**: [`CHANGELOG.md`](CHANGELOG.md)
- **Roadmap & Increments**: [`vdesign/plan.md`](vdesign/plan.md)
- **Session Checkpoint**: [`vdesign/checkpoint.md`](vdesign/checkpoint.md)
