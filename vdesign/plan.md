# Project Plan — cave

## Roadmap

- [ ] Phase 1: Sandbox Technology Evaluation & Specification
  - Compare isolation backends (`bwrap` vs. Docker/Podman vs. systemd-nspawn).
  - Define mount policies, environment variable handling, and credential isolation.
- [ ] Phase 2: Core CLI & Terminal Passthrough Prototype
  - Implement PTY allocation, raw mode handling, and command execution forwarding.
  - Verify seamless interactive UX with `claude` and other agent CLIs.
- [ ] Phase 3: Filesystem Isolation & Safety Boundaries
  - Enforce project directory scoping, read-only system roots, and masking of sensitive credentials.
- [ ] Phase 4: Agent Presets & Configuration
  - Built-in presets for Claude Code, Antigravity, Aider, and custom agent profiles.

## Candidates
<!-- Unnumbered DRAFT ideas awaiting validation -->
- Candidate: Network policy filtering (allowlist API endpoints, block local LAN/intranet).
- Candidate: Ephemeral git branch / copy-on-write snapshotting before agent execution.
