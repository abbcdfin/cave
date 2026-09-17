# Project Constraints

## Decision Making
1. **Consult first.** Check `./docs/`, `constraints.md`, `plan.md`, and `README.md` before concluding information is missing. Proceed if derivable.
2. **Ask before assuming.** Stop and ask for irreversible choices (architecture, data model, UX flow, technology choices, placement).
3. **Offer concrete options.** Frame questions as choices with explicit trade-offs.
4. **Record decisions.** Record answers and rationale in `./docs/` (design docs or ADRs).

## Output Discipline
- **Model first.** Express from a coherent system model rather than surface patching.
- **Concept naming.** Reuse system names; adjust the concept and propagate rather than coining duplicates.
- **Concise & true.** State what is true and what was done; eliminate preamble, restatement, and narration.
- **Specific.** Name files, line ranges, symbols, values, and trade-offs.
- **Earned length.** Say each thing once in its designated single source of truth.

## Lifecycle Constraints
- Work advances sequentially: `DRAFT` → `ACCEPTED` → `BUILDING` → `DONE` (skip nothing).
- Unvalidated candidates must remain **unnumbered** under `## Candidates` in `plan.md` or `./vdesign/proposals/`.
- Only accepted candidates become numbered phases in `plan.md`.
- Active phase work is tracked in `active_plan.md` and collapsed upon completion.
- Rejections must be recorded with their rationale rather than deleted.
- Deferred work must be recorded immediately (scoped inline TODOs or DRAFT candidates).
