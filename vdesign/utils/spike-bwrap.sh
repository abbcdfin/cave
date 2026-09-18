#!/usr/bin/env bash
# Throwaway bwrap spike script for cave
set -euo pipefail

REAL_USER="$(id -u)"
REAL_GROUP="$(id -g)"
REAL_HOME="$HOME"
REAL_CWD="$PWD"

BWRAP_ARGS=(
    # Die when parent exits
    --die-with-parent

    # Namespaces
    --unshare-pid
    --unshare-uts
    --unshare-ipc

    # Kernel & device virtual filesystems
    --proc /proc
    --dev /dev

    # Private ephemeral /tmp and /run
    --tmpfs /tmp
    --tmpfs /run

    # Base system (read-only)
    --ro-bind /usr /usr
    --ro-bind /etc /etc
)

# Optional system symlinks/dirs
for sysdir in /bin /sbin /lib /lib64 /lib32; do
    if [ -e "$sysdir" ]; then
        BWRAP_ARGS+=(--ro-bind "$sysdir" "$sysdir")
    fi
done

# Ephemeral private $HOME
BWRAP_ARGS+=(--tmpfs "$REAL_HOME")

# Read-only toolchain runtimes in $HOME
for ro_path in \
    "$REAL_HOME/.local/bin" \
    "$REAL_HOME/.local/share/claude" \
    "$REAL_HOME/.cargo/bin" \
    "$REAL_HOME/.npm_global" \
    "$REAL_HOME/.gitconfig"
do
    if [ -e "$ro_path" ]; then
        BWRAP_ARGS+=(--ro-bind "$ro_path" "$ro_path")
    fi
done

# Read-write agent state in $HOME
for rw_path in \
    "$REAL_HOME/.claude" \
    "$REAL_HOME/.claude.json"
do
    if [ -e "$rw_path" ]; then
        BWRAP_ARGS+=(--bind "$rw_path" "$rw_path")
    fi
done

# Current working directory (read-write at host path)
BWRAP_ARGS+=(
    --bind "$REAL_CWD" "$REAL_CWD"
    --chdir "$REAL_CWD"
)

# Set environment variables
BWRAP_ARGS+=(
    --setenv HOME "$REAL_HOME"
    --setenv USER "$USER"
    --setenv LOGNAME "$USER"
    --setenv PATH "$PATH"
    --setenv TERM "${TERM:-xterm-256color}"
)

[ -n "${COLORTERM:-}" ] && BWRAP_ARGS+=(--setenv COLORTERM "$COLORTERM")
[ -n "${LANG:-}" ] && BWRAP_ARGS+=(--setenv LANG "$LANG")
[ -n "${ANTHROPIC_API_KEY:-}" ] && BWRAP_ARGS+=(--setenv ANTHROPIC_API_KEY "$ANTHROPIC_API_KEY")

exec bwrap "${BWRAP_ARGS[@]}" "$@"
