# Product Specification — cave

## What & Why

### The Problem
Autonomous coding agents (e.g. Claude Code, Antigravity/AGY, Aider, Codex) are significantly faster when permitted to execute shell commands, edit files, and run tests without manual confirmation prompts for every action ("YOLO mode"). However, an agent running unattended on the developer's host makes mistakes whose damage reaches beyond the project:
- Reading, modifying, or deleting files outside the project (`~/.ssh`, `~/.aws`, other repositories, system directories).
- Process sprawl and untracked system modifications.

Conversely, manual confirmation prompts destroy developer flow and create cognitive fatigue, severely slowing down agentic development.

### The Solution
`cave` is a transparent sandbox runner for coding agents. It wraps agent invocations (e.g. `cave claude --dangerously-skip-permissions`), confining file access to the allowed directories, while retaining the exact interactive terminal UX (colors, key bindings, TTY/PTY ergonomics) of the host system.

## Scope

Decided 2026-09-17.

### Threat model: a clumsy agent
cave protects against an agent that *accidentally* reaches files outside the allowed directories — a wrong path, an overbroad `rm`, a global config write. It is not a defence against an agent deliberately escaping or exfiltrating data.

- **Allowed directories** — the working directory, the agent's own state (e.g. `~/.claude`), and any directory the user adds — are the only host files the agent can access: read-write, except directories the user adds read-only.
- **Host runtime** — the OS and toolchain paths the agent needs to run (`/usr`, `/etc`, …) — is readable, never writable.
- **Shared tool caches** (`~/.cache`, and caches such as `~/.cargo/registry` kept elsewhere) are read-write: they are disposable, and hiding them repeats every download.
- **Everything else is hidden**, including the rest of the home directory (`~/.config` too) and the host's `/tmp` and `/run` (the sandbox gets private ones).
- Ordinary commands that reach host files through a service count as file access, so those services are hidden too: the Docker socket (`docker run -v /:/host`) and the user session bus (`systemctl --user`).

Out of scope:
- Deliberate escapes: terminal input injection, `/proc` traversal, abstract-socket abuse.
- Network egress: the agent can send the contents of allowed directories, or its own credentials, anywhere. Filtering remains a candidate.
- Damage inside the allowed directories. Snapshotting remains a candidate.

### Platform
Linux only for now. Other platforms may follow.

### Invocation
`cave [cave-options] <agent> [args...]`: cave options come first; everything from the agent command on passes through unchanged.

- cave adds no agent flags. YOLO mode is enabled by the user passing the agent's own flag (`cave claude --dangerously-skip-permissions`), keeping that choice under the user's control.
- Extra allowed directories are added with `--dir PATH` (read-write) or `--ro-dir PATH` (read-only), repeatable. Every allowed directory appears at its host path, so relative paths between them, absolute paths inside projects, and agent state keyed by path (`~/.claude/projects/<cwd>/`) all stay valid. There are no custom target paths. An explicitly allowed path overrides the default for that path: `--dir /tmp` shows the host's `/tmp`, and `--ro-dir .` makes the working directory read-only. cave warns on `--dir /` and `--dir ~`, which undo the boundary. A config file may follow later.
- cave's own commands are options (`cave --doctor`), so no agent command name is reserved.

## Key Requirements

1. **Host-Identical Terminal UX**:
   - Full TTY/PTY forwarding, handling signals (`SIGINT`, `SIGWINCH`), escape sequences, and interactive prompts seamlessly.
2. **Transparent Invocations**:
   - Syntax mirrors host execution, as defined in Scope › Invocation.
3. **File Boundary**:
   - Enforces the boundary defined in Scope.
   - Agent state stays usable, so native login and session resume (`claude -c`) work.
4. **Minimal Overhead**:
   - Instant startup (< 100ms) to feel like a native local binary rather than a sluggish virtual machine.

## Supported Agents
- Claude Code (`claude`)
- Antigravity (`agy`)
- Aider (`aider`)
- General CLI / shell workflows
