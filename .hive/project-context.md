# Project Context — pycg-rs

## Overview
Rust reimplementation of PyCG — a static call-graph generator for Python, using ruff's parser for AST analysis.

## Architecture
- **`src/analyzer/`** (4,924 LOC) — Core analysis engine. Two-pass AST walk: pass 1 collects definitions/initial edges, resolves base classes and MRO between passes, pass 2 re-analyzes with inheritance, then postprocess (expand unknowns, contract non-existents, resolve imports, cull inherited, collapse inner scopes).
- **`src/node.rs`** — `Node` and `Flavor` types representing call-graph vertices (Module, Class, Function, Method, etc.).
- **`src/scope.rs`** — `ValueSet` abstraction for tracking what names may point to (union-on-rebind semantics).
- **`src/query.rs`** (1,322 LOC) — Query layer: callees, callers, neighbors, path, symbols-in, summary. Renders to JSON or text.
- **`src/writer/mod.rs`** (918 LOC) — Output serialization: DOT, TGF, text, JSON (with JSON schema contracts for each format).
- **`src/visgraph.rs`** — Visual graph intermediate representation for DOT/TGF/text output.
- **`src/main.rs`** — CLI via clap with subcommands: `analyze`, `symbols-in`, `summary`, `callees`, `callers`, `neighbors`, `path`.

**Data flow**: CLI collects Python file paths → `CallGraph::new()` runs the two-pass analyzer → query/writer layer formats output → stdout.

## Key Files
- `src/analyzer/mod.rs` — Main AST visitor, defines/uses edge construction (3,020 LOC, largest file)
- `src/analyzer/pipeline.rs` — `CallGraph::new()` entry point, `AnalysisSession` lifecycle
- `src/analyzer/postprocess.rs` — Unknown expansion, import resolution, edge culling
- `src/analyzer/resolution.rs` — Name resolution and attribute lookup
- `src/analyzer/mro.rs` — Method Resolution Order computation
- `src/analyzer/prepass.rs` — Pre-pass scope info extraction
- `src/analyzer/state.rs` — `AnalysisSession` struct definition, context management
- `src/node.rs` — `Node`, `Flavor`, `NodeId` types
- `src/query.rs` — All query subcommands implementation
- `src/writer/mod.rs` — All output format serializers
- `src/main.rs` — CLI argument parsing, file collection, dispatch
- `tests/integration/common.rs` — Test helpers (`make_call_graph`, `has_uses_edge`, etc.)
- `tests/fixtures/accuracy_cases.json` — 30 accuracy test cases with 63 expectations
- `docs/limitations.md` — Known analysis limitations
- `docs/roadmap.md` — Feature roadmap

## Build & Test
- **Language**: Rust, edition 2024 (uses `let_chains` syntax). Requires recent stable or nightly toolchain.
- **Package manager**: Cargo (binary: `pycg`, lib: `pycg_rs`)
- **Build**: `cargo build` (binary at `target/debug/pycg`)
- **Test**: `cargo test --all-targets` — 205 tests total (46 unit, 133 integration, 11 CLI, 10 JSON schema, 4 snapshot, 1 query schema; 3 corpus tests `#[ignore]`d). Corpus tests: `cargo test --test integration corpus -- --ignored`
- **Lint**: `cargo clippy --all-targets --all-features -- -D warnings`
- **Format**: `cargo fmt --all --check` (rustfmt)
- **Type check**: Rust compiler (no separate step)
- **Pre-commit**: N/A (CI enforces fmt + clippy + tests)
- **Quirks**: Corpus tests are `#[ignore]`d and require `scripts/bootstrap-corpora.sh --only-corpora` to clone 9 real Python repos into `benchmarks/corpora/`. CI runs these in a separate job.

## Conventions
- Modules use `mod.rs` pattern (not filename-based modules)
- `CallGraph` is the central public type; exposes `nodes_arena: Vec<Node>`, `defines_edges`, `uses_edges` as `HashMap<NodeId, HashSet<NodeId>>`, `defined: HashSet<NodeId>`
- `NodeId` is `usize` (index into `nodes_arena`)
- Error handling: `anyhow::Result` throughout; no custom error types
- Logging: `log` crate macros (`info!`, `debug!`) with `env_logger`
- Test fixtures: Python files in `tests/test_code/`, one per test scenario; test helpers in `tests/integration/common.rs`
- Integration tests organized by concern in `tests/integration/` (accuracy, core, features, corpus, etc.)
- JSON output has formal schemas in `docs/json-schema/`
- Two-letter CLI flags: `-d` defines, `-u` uses, `-c` colored, `-g` grouped, `-a` annotated, `-m` modules
- `pub(super)` for analyzer internals, `pub` for cross-crate API

## Dependencies & Integration
- **ruff_python_ast/parser/source_file/text_size** (pinned git rev `359981b`) — Python AST parsing; this is the foundational dependency
- **petgraph 0.7** — Graph algorithms (used in `visgraph.rs` for visual graph)
- **clap 4** (derive) — CLI argument parsing
- **walkdir 2** — Recursive directory traversal for Python file collection
- **serde/serde_json** — JSON serialization for output formats
- **anyhow** — Error handling
- **Dev**: `insta` (snapshot tests), `assert_cmd` (CLI tests), `jsonschema` (schema validation tests), `tempfile`
- No external services, databases, or network calls. Pure static analysis tool.

## Gotchas
- The ruff dependencies are pinned to a specific git revision — updating them may introduce breaking API changes since ruff's internal crates are not semver-stable.
- `analyzer/mod.rs` is 3,020 lines — the AST visitor handles every Python statement/expression type. Changes here require understanding the two-pass architecture.
- Nodes use arena indexing (`usize` into `Vec<Node>`), not pointers — node IDs are only valid within the same `CallGraph` instance.
- The postprocess phase order matters: `expand_unknowns` → `resolve_imports` → `contract_nonexistents` → `cull_inherited` → `collapse_inner`. Reordering will produce incorrect graphs.
- Test helpers (`has_uses_edge`, etc.) match by short name suffix, not FQN — use `has_uses_edge_full` for exact matching.
- The `PyCG/` and `pyan/` directories are vendored reference implementations, not part of this project's build.
- `mutants.out/` directories are cargo-mutants output, not project artifacts.
- Benchmarks require corpora to be cloned first (`scripts/bootstrap-corpora.sh`); results are `.gitignore`d.
