# Active Plan — Phase 8: Multi-agent sessions (`cave --profile claude,codex,agy tmux`)

Status: BUILDING. Exit criteria: see `vdesign/plan.md` › Phase 8 ("Done when").
Decisions & rationale: `docs/architecture_design.md` › Filesystem & Environment Policy › Agent sessions and Toolchains › Entries.

- [x] 8.1 (Commit 1 — Plan step 1) Add repeatable, comma-separated `--profile NAME` (`src/cli.rs`), `select_profiles` order & deduplication (`src/policy.rs`), suppress unknown-agent warning when `--profile` is given (`src/main.rs`), built-in `[profiles.codex]` with plain `.codex` (`src/profiles.toml`), and unit/boundary tests including a mutation check on applying a profile once.
- [ ] 8.2 (Commit 2 — Plan step 2) Route profile entries through `policy::resolve_entry` with explicit `MissingBehavior` (`src/policy.rs`), resolve all profiles/toolchains before `prepare_host_state` (`src/main.rs`), switch built-ins to `${CLAUDE_CONFIG_DIR:-.claude}`, `${bin:claude}/..`, and `${CODEX_HOME:-.codex}`, and add unit/boundary tests including a mutation check on the fatal `$HOME` case.
- [ ] 8.3 (Commit 3 — Plan step 3) Support comma-separated `--with` using the shared parser, add `cave --doctor --profile NAME` reusing toolchain doctor formatting (`src/cli.rs`), add tests, and measure three-profile startup overhead against the 100 ms budget.
- [ ] 8.4 (Commit 4 — Plan step 4) User live check with `cave --profile claude,codex,agy tmux`, reconcile `docs/architecture_design.md`, `docs/product_specification.md`, and `README.md`, and record outcome in `vdesign/plan.md`.
