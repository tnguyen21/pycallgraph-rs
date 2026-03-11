# Queen State

## User Goal
Clean up the test suite, add large-repo analysis coverage, improve call graph generation for Python constructs, and plan the next architecture-focused wave around abstract values, return propagation, imports, destructuring, and accuracy testing.

## Active Issues
- `w-bf8b696437d6`: Replace legacy `old_tests` regressions and remove `tests/old_tests` (`finalized`)
- `w-d05eec6e6b5f`: Add corpus-scale integration smoke tests for vendored Python repos (`finalized`)
- `w-b56b11d2ed01`: Implement iterator and context-manager protocol edges in the analyzer (`finalized`)
- `w-014789c0f4f0`: Refresh README TODOs and testing docs to match actual behavior (`finalized`)
- `w-3f59845f7dbd`: Build a golden accuracy harness from local `pyan/` and `PyCG/` references (`in_progress`, `worker-9d0550ba1a23`)
- `w-42d241e4837d`: Replace single-value bindings with abstract value sets (`open`, depends on `w-3f59845f7dbd`)
- `w-8280b5e510fb`: Add worklist-based call and return propagation (`open`, depends on `w-3f59845f7dbd`, `w-42d241e4837d`)
- `w-372f84ad24ee`: Upgrade import and reexport modeling (`open`, depends on `w-42d241e4837d`, `w-8280b5e510fb`)
- `w-b54c3ddd2ae5`: Fix destructuring and container/subscript flow (`open`, depends on `w-42d241e4837d`, `w-8280b5e510fb`)
- `w-f3edacd70df3`: Tighten residual semantics and unknown expansion (`open`, depends on `w-8280b5e510fb`, `w-372f84ad24ee`, `w-b54c3ddd2ae5`)
- `w-5111901ebaa3`: Fix call-result attribute resolution and multi-return propagation (`in_progress`, `worker-429d449515e6`)
- `w-f2ac46d84cbd`: Fix star-import privacy and `__all__` handling (`in_progress`, `worker-e875602bb053`)
- `w-6d1a5275fea1`: Audit and rewrite unsound accuracy tests (`in_progress`, `worker-b037771a4b55`)
- `w-dd2332ecd498`: Add explicit corpora bootstrap and ignore local reference dirs (`in_progress`, `worker-a81bf35b8582`)
- `w-79675a9f206a`: Resume architecture wave on corrected harness and call-resolution baseline (`open`, depends on `w-5111901ebaa3`, `w-f2ac46d84cbd`, `w-6d1a5275fea1`)

## Decisions Made
- Coordinate from `main` and use `hive --json` for Hive CLI commands.
- Do not create issues until the user approves a human-readable plan.
- Existing vendored corpora under `benchmarks/corpora/` include `requests`, `rich`, `flask`, `httpx`, and `black`.
- `tests/old_tests` is currently only referenced by three no-panic regression tests in `tests/integration.rs`.
- `tests/test_code/features.py` already contains fixtures for context managers, iterators, async iteration, `match`, comprehensions, starred unpacking, and other Python constructs.
- The current test suite passes after a clean rebuild; an initial failing run was caused by stale compiled artifacts embedding an old repo path.
- The README TODO list is partially stale because `super()` resolution code exists in `src/analyzer.rs`.
- The active Hive queue is the default `~/.hive/hive.db`; the repo-local `.hive/local-hive.db` was only a fallback while sandboxed and is not the live queue.
- The first wave has already landed or is landing: legacy test cleanup, vendored-corpus smoke tests, iterator/context-manager protocol edges, and README accuracy updates.
- The next major technical gap is the analyzer architecture itself: single-value bindings, opaque call results, shallow binding/import propagation, and weak accuracy coverage.
- The local `pyan/` and `PyCG/` trees are available as read-only reference clones for designing the accuracy harness; they must not be modified or committed.
- Post-merge review found follow-up correctness gaps: attribute resolution on direct call results is still broken, multi-return propagation is under-approximated, star-import privacy/`__all__` handling is unsound, and parts of the new accuracy harness currently lock in false positives or mislabel behavior.
- Auto-cloning/updating corpora during normal `cargo test` runs is likely the wrong default because it makes tests networked, stateful, and worktree-mutating; if adopted at all, it should be opt-in setup behavior rather than unconditional test-time mutation.
- The active follow-up batch now focuses on correctness and harness repair before more architecture work: call-result propagation, star-import soundness, accuracy-test audit, and explicit corpora bootstrap/ignore rules.

## Next Actions
- Monitor the four active follow-up workers for regressions or escalations.
- Keep the architecture continuation (`w-79675a9f206a`) blocked until the correctness/harness trio is finalized.
