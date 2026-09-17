# Product Specification — cave

## What & Why

### The Problem
Autonomous coding agents (e.g. Claude Code, Antigravity/AGY, Aider, Codex) are significantly faster when permitted to execute shell commands, edit files, and run tests without manual confirmation prompts for every action ("YOLO mode"). However, running unvetted LLM agent commands directly on the developer's host machine introduces serious risks:
- Accidental destruction or modification of files outside the project tree (`~/.ssh`, `~/.aws`, system directories).
- Untrusted script execution or credential leakage.
- Process sprawl and untracked system modifications.

Conversely, manual confirmation prompts destroy developer flow and create cognitive fatigue, severely slowing down agentic development.

### The Solution
`cave` is a transparent sandbox runner for coding agents. It wraps agent invocations (e.g. `cave claude -c`), confining execution to an isolated, least-privilege environment scoped strictly to the current workspace, while retaining the exact interactive terminal UX (colors, key bindings, TTY/PTY ergonomics) of the host system.

## Key Requirements

1. **Host-Identical Terminal UX**:
   - Full TTY/PTY forwarding, handling signals (`SIGINT`, `SIGWINCH`), escape sequences, and interactive prompts seamlessly.
2. **Transparent Invocations**:
   - Syntax mirrors host execution: `cave <agent> [args...]`.
3. **Restricted Boundary**:
   - Workspace isolation: Read/write restricted to current workspace/project directory.
   - Host isolation: Sensitive host directories (`~/.ssh`, `~/.gnupg`, `/etc`, host root) blocked or read-only mock.
   - Config & Auth Passthrough: Controlled forwarding of required API keys and agent configurations without exposing raw host secrets.
4. **Minimal Overhead**:
   - Instant startup (< 100ms) to feel like a native local binary rather than a sluggish virtual machine.

## Supported Agents
- Claude Code (`claude`)
- Antigravity (`agy`)
- Aider (`aider`)
- General CLI / shell workflows
