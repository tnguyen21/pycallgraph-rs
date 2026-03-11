# Temporary Plan

This file is intentionally temporary. It exists to turn the repo review into
concrete execution work.

## Immediate

- [x] Add real CI for pull requests and pushes.
- [x] Fix the GitHub Pages/report workflow so published metrics are accurate.
- [x] Update README installation, quickstart, JSON docs, and project positioning.
- [x] Repair benchmark reproducibility by checking in the harness referenced by docs.
- [x] Fix current Clippy warnings so CI can enforce `-D warnings`.

## Near Term

- [x] Split `src/analyzer/mod.rs` into clearer pass/resolution modules.
- [x] Replace return-propagation full-map cloning with dirty tracking or a worklist.
- [ ] Tighten permissive integration helpers and add a few stronger corpus invariants.
- [ ] Decide whether the project is optimizing first for library consumers or CLI users.

## Later

- [ ] Add real benchmark trend tracking over release builds.
- [ ] Stabilize and document the JSON schema if library/integration usage is a goal.
- [ ] Add release/distribution automation once the install story is settled.

## Next Slice

- [ ] Extract the next analyzer helper slab without touching the statement-visitor spine.
- [ ] Add stronger corpus assertions beyond non-degenerate graph checks.
- [ ] Tighten integration helper matching so tests assert exact fully-qualified targets.
