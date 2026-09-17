# cave

Run coding agents autonomously in **YOLO mode** without putting your host system at risk.

`cave` transparently sandboxes coding agents (such as Claude Code, Antigravity, Aider) in a restricted environment so you can run them with auto-approvals enabled, while preserving the exact interactive terminal experience as if running directly on your host.

## Why cave?

- **Frictionless Autonomy (YOLO Mode)**: Eliminates repetitive permission prompts without exposing your entire host system, credentials, or personal files to unintended agent actions.
- **Identical Terminal UX**: Transparently proxies PTY, interactive input, colors, keybindings, and window sizing.
- **Seamless Working Directory**: Automatically mounts and maps the current working directory, preserving file ownership and git context.

## Planned Workflow

Invoke `cave` followed by the agent CLI command and its arguments:

```bash
# Launch Claude Code in YOLO / auto-approve mode inside the cave sandbox
cave claude -c

# Launch other agents with their respective arguments
cave agy
cave aider --architect
```

The terminal opens directly into the agent session in your current working directory.

## CLI Syntax

```bash
cave [cave-options] <agent-command> [agent-arguments...]
```

### Common Commands

| Invocation | Action |
| :--- | :--- |
| `cave claude -c` | Runs Claude Code with auto-continue in the sandboxed CWD |
| `cave <agent>` | Runs the specified agent command inside the isolated environment |
| `cave doctor` | Inspects sandbox backend availability and permissions |

## Architecture & Roadmap

- **Product Specification (What & Why)**: [`docs/product_specification.md`](docs/product_specification.md)
- **Architecture Design (How)**: [`docs/architecture_design.md`](docs/architecture_design.md)
- **Roadmap & Phases**: [`vdesign/plan.md`](vdesign/plan.md)
- **Session Checkpoint**: [`vdesign/checkpoint.md`](vdesign/checkpoint.md)
