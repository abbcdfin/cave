# Checkpoint

## Current State
- Merged `docs/scope-and-roadmap` into `main`.
- Started Phase 1: marked in progress in [`plan.md`](plan.md).
- Created [`active_plan.md`](active_plan.md) and completed Task 1.1:
  - `bwrap` 0.11.2 verified at `/usr/bin/bwrap`.
  - Unprivileged user namespaces verified enabled (`/proc/sys/kernel/unprivileged_userns_clone` = 1).
  - Host `claude` binary verified at `/home/jian/.local/bin/claude` (symlink to `/home/jian/.local/share/claude/versions/2.1.274`).
  - Existing agent state verified: `~/.claude/` (directory) and `~/.claude.json` (file).

## Next Steps
- Task 1.2: Classify home directory paths into agent state (RW), toolchain runtime (RO), and hidden.
