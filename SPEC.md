# pyan-rs: Rust Port of pyan3

## Goal

A Rust reimplementation of pyan3's core call graph analysis for Python programs.
MVP: analyze a set of `.py` files and produce a DOT-format call graph showing
defines and uses edges between functions, classes, methods, and modules.

## Architecture

```
┌─────────────┐     ┌──────────────┐     ┌────────────┐     ┌──────────┐
│ File Walker  │────▶│ AST Parser   │────▶│  Analyzer  │────▶│ Writers  │
│ (walkdir)    │     │(ruff_python) │     │ (visitor)  │     │ (dot,tgf)│
└─────────────┘     └──────────────┘     └────────────┘     └──────────┘
                                               │
                                         ┌─────┴─────┐
                                         │   Graph   │
                                         │(petgraph) │
                                         └───────────┘
```

### Modules

- `node.rs` — `Node` struct + `Flavor` enum (FUNCTION, METHOD, CLASS, MODULE, etc.)
- `scope.rs` — Scope/binding tracking (replaces Python's `symtable` usage)
- `analyzer.rs` — AST visitor, two-pass analysis, edge construction
- `graph.rs` — Call graph data structure (defines_edges, uses_edges)
- `writer/dot.rs` — DOT format output
- `writer/tgf.rs` — TGF format output
- `writer/text.rs` — Plain text output
- `visgraph.rs` — Visual graph representation with coloring
- `main.rs` — CLI (clap)

### Key Design Decisions

1. **AST Parsing**: Use `ruff_python_parser` — same parser that powers ruff.
   Avoids any Python runtime dependency.

2. **Scope Analysis**: Reimplement in Rust. Python's `symtable` module is CPython-specific
   and calling it via FFI defeats the purpose. We only need: which names are local vs
   free, what's defined in each scope.

3. **Graph Storage**: Use `petgraph::DiGraph` for the underlying graph, with separate
   edge types for "defines" vs "uses" relationships.

4. **Two-Pass Analysis**: Same strategy as Python pyan:
   - Pass 1: Collect all definitions, build initial edges
   - Between passes: Resolve base classes, compute MRO
   - Pass 2: Re-analyze with full inheritance info

## MVP Scope

### In scope
- Parse Python 3.10+ source files
- Track: module, class, function/method definitions
- Track: function calls, attribute access, imports
- Handle: relative imports, import aliases
- Handle: `self` binding in methods, basic MRO
- Output: DOT format
- CLI: accept file globs, output to stdout

### Out of scope (for now)
- PyO3 bindings
- SVG/HTML output (requires graphviz)
- yEd GraphML output
- Sphinx plugin
- Module-level analysis mode
- Match statements, walrus operator, type aliases
- Starred unpacking resolution
- Context manager protocol edges (__enter__/__exit__)

## Test Strategy

Port key test cases from Python pyan:
1. Basic function/class definition tracking
2. Import resolution (absolute + relative)
3. Method call resolution through self
4. Inheritance / MRO
5. DOT output format validation
6. End-to-end: analyze test fixtures, verify edges exist
