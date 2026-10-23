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
| `cave --no-ssh <agent>` | Disables forwarding the host SSH agent socket for this session |
| `cave --dir ../shared-lib claude` | Also allows read-write access to `../shared-lib` |
| `cave --ro-dir ~/reference claude` | Also allows read-only access to `~/reference` |
| `cave --doctor` | Inspects sandbox backend availability and permissions |

## Configuration

`cave` works out of the box with embedded profiles for supported agents (`claude`, `agy`, `aider`, `sh`, `bash`, `zsh`).

### SSH Agent Forwarding

`cave` forwards the host's `ssh-agent` socket by default to enable git operations over SSH without disclosing private credentials:

- **Socket Forwarding**: Bound read-only at `/run/user/$UID/ssh-agent.sock` within a private runtime directory (`0700`).
- **Credential Isolation**: Private keys (`~/.ssh/id_*`) remain strictly hidden and are never mounted into the sandbox.
- **Config & Known Hosts**: `~/.ssh/config` and `~/.ssh/known_hosts` are mounted read-only by default when present on the host.
- **Host Drop-ins (`/etc/ssh/ssh_config.d`)**: System drop-in directories are masked with an empty tmpfs so OpenSSH does not reject host root-owned files (which appear as `nobody` inside unprivileged user namespaces). Host-wide custom SSH configurations belong in `~/.ssh/config`.
- **Opt-out**: Pass `--no-ssh` before the agent command (`cave --no-ssh <agent>`) to disable forwarding for that session.
- **Stale Sockets**: If `SSH_AUTH_SOCK` points to a missing or non-socket path, `cave` emits a warning to stderr and unsets the variable inside the sandbox without failing the session.

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
```

- **Layering**: User configuration appends to embedded defaults. Re-declaring an existing path moves it to the new class (e.g., downgrading `.cache` to `ro_home` makes it read-only).
- **Isolation**: `~/.config/cave/` is hidden by default (since `$HOME` is an empty private tmpfs without `.config` mounted). It is visible only if you explicitly mount a parent directory (e.g. `--dir ~/.config`).
- **Diagnostics**: Run `cave --doctor` to inspect loaded configuration sources and the effective merged table.

## Architecture & Roadmap

- **Product Specification (What & Why)**: [`docs/product_specification.md`](docs/product_specification.md)
- **Architecture Design (How)**: [`docs/architecture_design.md`](docs/architecture_design.md)
- **Roadmap & Phases**: [`vdesign/plan.md`](vdesign/plan.md)
- **Session Checkpoint**: [`vdesign/checkpoint.md`](vdesign/checkpoint.md)
