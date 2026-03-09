//! Integration tests for pyan-rs.
//!
//! Uses Python test fixtures in tests/test_code/.

use std::collections::HashSet;
use std::path::PathBuf;

use pycallgraph_rs::analyzer::CallGraph;
use pycallgraph_rs::visgraph::{VisualGraph, VisualOptions};
use pycallgraph_rs::writer;

fn test_code_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("test_code")
}

fn collect_py_files(dir: &std::path::Path) -> Vec<String> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().is_some_and(|ext| ext == "py")
                && !e.path().to_string_lossy().contains("__pycache__")
        })
    {
        files.push(entry.path().to_string_lossy().to_string());
    }
    files.sort();
    files
}

fn make_call_graph(dir: &std::path::Path) -> CallGraph {
    let files = collect_py_files(dir);
    let root = dir.parent().unwrap().to_string_lossy().to_string();
    CallGraph::new(&files, Some(&root)).expect("analysis should succeed")
}

/// Find all nodes with the given short name, or whose fully qualified name ends with the given name.
fn find_nodes_by_name(cg: &CallGraph, name: &str) -> Vec<usize> {
    let mut result: Vec<usize> = cg.nodes_by_name
        .get(name)
        .cloned()
        .unwrap_or_default();
    for (idx, node) in cg.nodes_arena.iter().enumerate() {
        if node.get_name() == name || node.get_name().ends_with(&format!(".{name}")) {
            if !result.contains(&idx) {
                result.push(idx);
            }
        }
    }
    result
}

/// Get the set of short names that `source_name` defines.
fn get_defines(cg: &CallGraph, source_name: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    for &nid in find_nodes_by_name(cg, source_name).iter() {
        if let Some(targets) = cg.defines_edges.get(&nid) {
            for &tid in targets {
                result.insert(cg.nodes_arena[tid].name.clone());
            }
        }
    }
    result
}

/// Get the set of short names that `source_name` uses.
fn get_uses(cg: &CallGraph, source_name: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    for &nid in find_nodes_by_name(cg, source_name).iter() {
        if let Some(targets) = cg.uses_edges.get(&nid) {
            for &tid in targets {
                result.insert(cg.nodes_arena[tid].name.clone());
            }
        }
    }
    result
}

/// Check if there is a defines edge from a node matching `from_name` to one matching `to_name`.
fn has_defines_edge(cg: &CallGraph, from_name: &str, to_name: &str) -> bool {
    for &fid in find_nodes_by_name(cg, from_name).iter() {
        if let Some(targets) = cg.defines_edges.get(&fid) {
            for &tid in targets {
                if cg.nodes_arena[tid].name == to_name {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if there is a uses edge from a node matching `from_name` to one matching `to_name`.
fn has_uses_edge(cg: &CallGraph, from_name: &str, to_name: &str) -> bool {
    for &fid in find_nodes_by_name(cg, from_name).iter() {
        if let Some(targets) = cg.uses_edges.get(&fid) {
            for &tid in targets {
                if cg.nodes_arena[tid].name == to_name {
                    return true;
                }
            }
        }
    }
    false
}

// ===================================================================
// Core analysis tests
// ===================================================================

#[test]
fn test_modules_found() {
    let cg = make_call_graph(&test_code_dir());
    let module_names: Vec<_> = cg.nodes_arena.iter()
        .filter(|n| n.flavor == pycallgraph_rs::node::Flavor::Module)
        .map(|n| n.get_name())
        .collect();
    assert!(module_names.iter().any(|n| n.contains("submodule1")), "submodule1 not found");
    assert!(module_names.iter().any(|n| n.contains("submodule2")), "submodule2 not found");
}

#[test]
fn test_class_found() {
    let cg = make_call_graph(&test_code_dir());
    let classes: Vec<_> = cg.nodes_arena.iter()
        .filter(|n| n.flavor == pycallgraph_rs::node::Flavor::Class)
        .map(|n| n.name.clone())
        .collect();
    assert!(classes.contains(&"A".to_string()), "Class A not found, got: {:?}", classes);
}

#[test]
fn test_function_found() {
    let cg = make_call_graph(&test_code_dir());
    let functions: Vec<_> = cg.nodes_arena.iter()
        .filter(|n| matches!(n.flavor, pycallgraph_rs::node::Flavor::Function | pycallgraph_rs::node::Flavor::Method))
        .map(|n| n.name.clone())
        .collect();
    assert!(functions.contains(&"test_func1".to_string()), "test_func1 not found, got: {:?}", functions);
}

#[test]
fn test_submodule_defines() {
    let cg = make_call_graph(&test_code_dir());
    let defs = get_defines(&cg, "submodule2");
    assert!(defs.contains("test_2"), "submodule2 should define test_2, got: {:?}", defs);
}

#[test]
fn test_uses_edge_exists() {
    let cg = make_call_graph(&test_code_dir());
    let uses = get_uses(&cg, "test_2");
    assert!(
        uses.contains("test_func1") || uses.contains("test_func2"),
        "test_2 should use test_func1 or test_func2, got: {:?}", uses
    );
}

// ===================================================================
// DOT output format tests
// ===================================================================

#[test]
fn test_dot_output_valid() {
    let cg = make_call_graph(&test_code_dir());
    let opts = VisualOptions {
        draw_defines: true,
        draw_uses: true,
        colored: true,
        grouped: false,
        annotated: false,
    };
    let vg = VisualGraph::from_call_graph(
        &cg.nodes_arena, &cg.defined, &cg.defines_edges, &cg.uses_edges, &opts,
    );
    let dot = writer::write_dot(&vg, &["rankdir=TB".to_string()]);
    assert!(dot.starts_with("digraph G {"), "DOT output should start with 'digraph G {{'");
    assert!(dot.trim().ends_with('}'), "DOT output should end with '}}'");
    assert!(dot.contains("->"), "DOT output should contain edges");
    assert!(dot.contains("style=\"dashed\""), "DOT output should have defines edges (dashed)");
    assert!(dot.contains("style=\"solid\""), "DOT output should have uses edges (solid)");
}

#[test]
fn test_dot_output_grouped() {
    let cg = make_call_graph(&test_code_dir());
    let opts = VisualOptions {
        draw_defines: true,
        draw_uses: true,
        colored: true,
        grouped: true,
        annotated: false,
    };
    let vg = VisualGraph::from_call_graph(
        &cg.nodes_arena, &cg.defined, &cg.defines_edges, &cg.uses_edges, &opts,
    );
    let dot = writer::write_dot(&vg, &["rankdir=TB".to_string()]);
    assert!(dot.contains("subgraph cluster_"), "Grouped DOT should have subgraphs");
}

// ===================================================================
// TGF output format tests
// ===================================================================

#[test]
fn test_tgf_output_valid() {
    let cg = make_call_graph(&test_code_dir());
    let opts = VisualOptions {
        draw_defines: true,
        draw_uses: true,
        colored: false,
        grouped: false,
        annotated: false,
    };
    let vg = VisualGraph::from_call_graph(
        &cg.nodes_arena, &cg.defined, &cg.defines_edges, &cg.uses_edges, &opts,
    );
    let tgf = writer::write_tgf(&vg);
    assert!(tgf.contains('#'), "TGF should have # separator");
    let parts: Vec<&str> = tgf.splitn(2, '#').collect();
    assert_eq!(parts.len(), 2);
    let edges_section = parts[1].trim();
    assert!(!edges_section.is_empty(), "TGF should have edges");
}

// ===================================================================
// Text output format tests
// ===================================================================

#[test]
fn test_text_output_valid() {
    let cg = make_call_graph(&test_code_dir());
    let opts = VisualOptions {
        draw_defines: true,
        draw_uses: true,
        colored: false,
        grouped: false,
        annotated: false,
    };
    let vg = VisualGraph::from_call_graph(
        &cg.nodes_arena, &cg.defined, &cg.defines_edges, &cg.uses_edges, &opts,
    );
    let text = writer::write_text(&vg);
    assert!(text.contains("[D]") || text.contains("[U]"), "Text should have tagged edges");
    for line in text.lines() {
        if line.starts_with("    ") {
            assert!(
                line.contains("[D]") || line.contains("[U]"),
                "Indented lines should be tagged edges: {line}"
            );
        }
    }
}

// ===================================================================
// Regression: don't crash on edge cases
// ===================================================================

/// Issue #2: annotated assignments at module level (`a: int = 3`) must not
/// crash the analyzer.
#[test]
fn test_regression_annotated_assignments() {
    let fixture = test_code_dir().join("regression_issue2.py");
    let files = vec![fixture.to_string_lossy().to_string()];
    let cg = CallGraph::new(&files, None)
        .expect("issue2: annotated assignment must not crash the analyzer");
    // The file defines annotated_fn and Container – verify we produced nodes.
    assert!(!cg.nodes_arena.is_empty(), "issue2: graph should not be empty");
    let fn_names: Vec<_> = cg.nodes_arena.iter()
        .filter(|n| matches!(n.flavor,
            pycallgraph_rs::node::Flavor::Function | pycallgraph_rs::node::Flavor::Method))
        .map(|n| n.name.as_str())
        .collect();
    assert!(fn_names.contains(&"annotated_fn"),
        "issue2: annotated_fn not found, got: {fn_names:?}");
}

/// Issue #3: complex / nested comprehensions (list-inside-list, dict-in-list,
/// generator-as-iterable) must not crash the analyzer.
#[test]
fn test_regression_comprehensions() {
    let fixture = test_code_dir().join("regression_issue3.py");
    let files = vec![fixture.to_string_lossy().to_string()];
    let cg = CallGraph::new(&files, None)
        .expect("issue3: comprehensions must not crash the analyzer");
    let fn_names: Vec<_> = cg.nodes_arena.iter()
        .filter(|n| matches!(n.flavor,
            pycallgraph_rs::node::Flavor::Function | pycallgraph_rs::node::Flavor::Method))
        .map(|n| n.name.as_str())
        .collect();
    assert!(fn_names.contains(&"f"), "issue3: function f not found, got: {fn_names:?}");
    assert!(fn_names.contains(&"g"), "issue3: function g not found, got: {fn_names:?}");
    assert!(fn_names.contains(&"h"), "issue3: function h not found, got: {fn_names:?}");
}

/// Issue #5: files that reference external / uninstalled packages (numpy,
/// pandas) and relative imports whose targets don't exist must not crash.
#[test]
fn test_regression_external_deps() {
    let fixture = test_code_dir().join("regression_issue5.py");
    let files = vec![fixture.to_string_lossy().to_string()];
    let cg = CallGraph::new(&files, None)
        .expect("issue5: external-dep imports must not crash the analyzer");
    let class_names: Vec<_> = cg.nodes_arena.iter()
        .filter(|n| n.flavor == pycallgraph_rs::node::Flavor::Class)
        .map(|n| n.name.as_str())
        .collect();
    assert!(class_names.contains(&"MyProcessor"),
        "issue5: MyProcessor not found, got: {class_names:?}");
}

// ===================================================================
// Feature coverage (features.py)
// ===================================================================

fn make_features_graph() -> CallGraph {
    let features_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("test_code")
        .join("features.py");
    let files = vec![features_file.to_string_lossy().to_string()];
    CallGraph::new(&files, None).expect("should parse features.py")
}

#[test]
fn test_features_classes_found() {
    let cg = make_features_graph();
    let class_names: HashSet<_> = cg.nodes_arena.iter()
        .filter(|n| n.flavor == pycallgraph_rs::node::Flavor::Class)
        .map(|n| n.name.as_str())
        .collect();
    for expected in ["Decorated", "Base", "Derived", "MixinA", "MixinB", "Combined"] {
        assert!(class_names.contains(expected), "Class {expected} not found, got: {class_names:?}");
    }
}

#[test]
fn test_features_decorators() {
    let cg = make_features_graph();
    assert!(has_defines_edge(&cg, "Decorated", "static_method"));
    assert!(has_defines_edge(&cg, "Decorated", "class_method"));
    assert!(has_defines_edge(&cg, "Decorated", "my_prop"));
    assert!(has_defines_edge(&cg, "Decorated", "regular"));

    let sm: Vec<_> = find_nodes_by_name(&cg, "static_method").into_iter()
        .filter(|&id| cg.nodes_arena[id].flavor == pycallgraph_rs::node::Flavor::StaticMethod)
        .collect();
    assert!(!sm.is_empty(), "static_method should have StaticMethod flavor");

    let cm: Vec<_> = find_nodes_by_name(&cg, "class_method").into_iter()
        .filter(|&id| cg.nodes_arena[id].flavor == pycallgraph_rs::node::Flavor::ClassMethod)
        .collect();
    assert!(!cm.is_empty(), "class_method should have ClassMethod flavor");
}

#[test]
fn test_features_inheritance() {
    let cg = make_features_graph();
    assert!(has_uses_edge(&cg, "Derived", "Base"),
            "Derived should use Base (inheritance)");
    assert!(has_uses_edge(&cg, "bar", "foo"),
            "bar should use foo");
}

#[test]
fn test_features_multiple_inheritance() {
    let cg = make_features_graph();
    assert!(has_uses_edge(&cg, "Combined", "MixinA"),
            "Combined should use MixinA");
    assert!(has_uses_edge(&cg, "Combined", "MixinB"),
            "Combined should use MixinB");
}

// ===================================================================
// INV-1: iterator protocol edges
// ===================================================================

/// `iterate_sequence` must gain uses edges to `__iter__` and `__next__`
/// when iterating over a `Sequence()` instance in a `for` loop.
#[test]
fn test_iterator_protocol_for_loop() {
    let cg = make_features_graph();
    assert!(
        has_uses_edge(&cg, "iterate_sequence", "__iter__"),
        "iterate_sequence should use Sequence.__iter__ (for-loop protocol)"
    );
    assert!(
        has_uses_edge(&cg, "iterate_sequence", "__next__"),
        "iterate_sequence should use Sequence.__next__ (for-loop protocol)"
    );
}

/// `comprehend_sequence` must gain the same iterator protocol edges
/// because the comprehension iterates over `Sequence()`.
#[test]
fn test_iterator_protocol_comprehension() {
    let cg = make_features_graph();
    assert!(
        has_uses_edge(&cg, "comprehend_sequence", "__iter__"),
        "comprehend_sequence should use Sequence.__iter__ (comprehension protocol)"
    );
    assert!(
        has_uses_edge(&cg, "comprehend_sequence", "__next__"),
        "comprehend_sequence should use Sequence.__next__ (comprehension protocol)"
    );
}

/// Protocol edges must only be emitted for known-class iterables, not for
/// unknown/unresolved iterables (e.g., function arguments like `items`).
#[test]
fn test_iterator_protocol_not_emitted_for_unknowns() {
    let cg = make_features_graph();
    // process_items(items) iterates over an argument — we must NOT emit
    // protocol edges from unknown/argument nodes.
    let uses = get_uses(&cg, "process_items");
    assert!(
        !uses.contains("__iter__"),
        "process_items iterates an arg, should NOT produce __iter__ edge, got: {uses:?}"
    );
    assert!(
        !uses.contains("__next__"),
        "process_items iterates an arg, should NOT produce __next__ edge, got: {uses:?}"
    );
}

// ===================================================================
// INV-2: context-manager protocol edges
// ===================================================================

/// `use_ctx` must gain uses edges to `__enter__` and `__exit__`
/// when entering a `with MyCtx()` block.
#[test]
fn test_context_manager_protocol_sync() {
    let cg = make_features_graph();
    assert!(
        has_uses_edge(&cg, "use_ctx", "__enter__"),
        "use_ctx should use MyCtx.__enter__ (with-statement protocol)"
    );
    assert!(
        has_uses_edge(&cg, "use_ctx", "__exit__"),
        "use_ctx should use MyCtx.__exit__ (with-statement protocol)"
    );
}

/// `use_async_cm` must gain uses edges to `__aenter__` and `__aexit__`
/// when entering an `async with AsyncCM()` block.
#[test]
fn test_context_manager_protocol_async() {
    let cg = make_features_graph();
    assert!(
        has_uses_edge(&cg, "use_async_cm", "__aenter__"),
        "use_async_cm should use AsyncCM.__aenter__ (async with protocol)"
    );
    assert!(
        has_uses_edge(&cg, "use_async_cm", "__aexit__"),
        "use_async_cm should use AsyncCM.__aexit__ (async with protocol)"
    );
}

/// No wildcard unknown nodes should appear for the protocol method names.
/// If we see `*.____iter__` or `*.__enter__` etc., we resolved wrong.
#[test]
fn test_protocol_edges_resolve_to_known_nodes() {
    let cg = make_features_graph();
    // All nodes for __iter__ / __next__ / __enter__ / __exit__ must have a
    // non-None namespace (i.e., be concrete, not wildcard).
    let protocol_methods = ["__iter__", "__next__", "__enter__", "__exit__"];
    for method in protocol_methods {
        for &nid in cg.nodes_by_name.get(method).unwrap_or(&vec![]) {
            assert!(
                cg.nodes_arena[nid].namespace.is_some(),
                "Protocol method {method} resolved to a wildcard node — expected concrete"
            );
        }
    }
}

// ===================================================================
// INV-3: existing feature coverage must stay green
// ===================================================================

/// Existing decorator, inheritance, and match coverage must not regress.
#[test]
fn test_features_async_iterator_protocol() {
    let cg = make_features_graph();
    // `iterate_async_stream` is async-for over `AsyncStream()`.
    assert!(
        has_uses_edge(&cg, "iterate_async_stream", "__aiter__"),
        "iterate_async_stream should use AsyncStream.__aiter__"
    );
    assert!(
        has_uses_edge(&cg, "iterate_async_stream", "__anext__"),
        "iterate_async_stream should use AsyncStream.__anext__"
    );
}

// ===================================================================
// Performance
// ===================================================================

#[test]
fn test_performance() {
    let dir = test_code_dir();
    let files = collect_py_files(&dir);
    let root = dir.parent().unwrap().to_string_lossy().to_string();

    let start = std::time::Instant::now();
    for _ in 0..100 {
        let _ = CallGraph::new(&files, Some(&root)).unwrap();
    }
    let elapsed = start.elapsed();
    let per_run = elapsed / 100;
    eprintln!("Average analysis time: {:?} (100 runs over {} files)", per_run, files.len());
    assert!(per_run.as_millis() < 200, "Analysis too slow: {:?}", per_run);
}

// ===================================================================
// Corpus-scale integration smoke tests
//
// Run the analyzer against real-world vendored Python packages from
// benchmarks/corpora/ and assert the resulting graph is non-degenerate.
//
// Tests skip (pass with a notice) when the corpus directory is absent
// (e.g. a fresh clone without vendored corpora), so the suite remains
// green in CI.  They fail if the directory IS present but analysis
// produces an empty or near-empty graph, which would indicate a
// regression.
// ===================================================================

/// Resolve the path to a specific package subdirectory inside the vendored
/// corpora.  Returns `None` if the directory does not exist (e.g. the
/// corpora have not been downloaded).
fn corpus_dir(package: &str, subpath: &str) -> Option<std::path::PathBuf> {
    let candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benchmarks")
        .join("corpora")
        .join(package)
        .join(subpath);
    if candidate.is_dir() {
        Some(candidate)
    } else {
        None
    }
}

/// Counts of the major node/edge kinds after analysis.
struct CorpusStats {
    modules: usize,
    classes: usize,
    functions: usize,
    uses_edge_count: usize,
}

/// Run the full analysis pipeline over `dir` and return summary stats.
///
/// Panics (test failure) if:
/// - no `.py` files are found in the directory
/// - `CallGraph::new` returns an error
fn analyze_corpus(dir: &std::path::Path) -> (CallGraph, CorpusStats) {
    let files = collect_py_files(dir);
    assert!(
        !files.is_empty(),
        "No Python files found in {dir:?} — corpus may be empty or mis-configured"
    );

    let root = dir.parent().unwrap().to_string_lossy().to_string();
    let cg = CallGraph::new(&files, Some(&root))
        .unwrap_or_else(|e| panic!("corpus analysis of {dir:?} failed: {e}"));

    let modules = cg
        .nodes_arena
        .iter()
        .filter(|n| n.flavor == pycallgraph_rs::node::Flavor::Module)
        .count();
    let classes = cg
        .nodes_arena
        .iter()
        .filter(|n| n.flavor == pycallgraph_rs::node::Flavor::Class)
        .count();
    let functions = cg
        .nodes_arena
        .iter()
        .filter(|n| {
            matches!(
                n.flavor,
                pycallgraph_rs::node::Flavor::Function
                    | pycallgraph_rs::node::Flavor::Method
                    | pycallgraph_rs::node::Flavor::StaticMethod
                    | pycallgraph_rs::node::Flavor::ClassMethod
            )
        })
        .count();
    let uses_edge_count: usize = cg.uses_edges.values().map(|s| s.len()).sum();

    eprintln!(
        "[corpus {dir:?}] {} files → {} modules, {} classes, {} functions, {} uses edges",
        files.len(),
        modules,
        classes,
        functions,
        uses_edge_count
    );

    (cg, CorpusStats { modules, classes, functions, uses_edge_count })
}

/// Assert that `stats` meets the provided lower bounds.  All bounds must be
/// conservative enough that a healthy analysis always clears them.
fn assert_corpus_healthy(
    label: &str,
    stats: &CorpusStats,
    min_modules: usize,
    min_classes: usize,
    min_functions: usize,
    min_uses_edges: usize,
) {
    assert!(
        stats.modules >= min_modules,
        "{label}: expected ≥{min_modules} module nodes, got {}",
        stats.modules
    );
    assert!(
        stats.classes >= min_classes,
        "{label}: expected ≥{min_classes} class nodes, got {}",
        stats.classes
    );
    assert!(
        stats.functions >= min_functions,
        "{label}: expected ≥{min_functions} function/method nodes, got {}",
        stats.functions
    );
    assert!(
        stats.uses_edge_count >= min_uses_edges,
        "{label}: expected ≥{min_uses_edges} uses edges, got {}",
        stats.uses_edge_count
    );
}

/// Smoke test: analyze the `requests` package (~18 files).
///
/// Conservative lower bounds chosen so that an empty/degenerate graph
/// fails while leaving headroom for refactors that remove some nodes.
#[test]
fn test_corpus_requests() {
    let Some(dir) = corpus_dir("requests", "src/requests") else {
        eprintln!("SKIP test_corpus_requests: benchmarks/corpora/requests/src/requests not found");
        return;
    };

    let (_, stats) = analyze_corpus(&dir);

    // requests has 18 source files, ~9 classes, many dozens of functions
    assert_corpus_healthy("requests", &stats, 10, 5, 20, 15);
}

/// Smoke test: analyze the `rich` package (~78 files).
#[test]
fn test_corpus_rich() {
    let Some(dir) = corpus_dir("rich", "rich") else {
        eprintln!("SKIP test_corpus_rich: benchmarks/corpora/rich/rich not found");
        return;
    };

    let (_, stats) = analyze_corpus(&dir);

    // rich has 78 source files, 50+ classes, 150+ methods/functions
    assert_corpus_healthy("rich", &stats, 40, 30, 80, 60);
}

/// Smoke test: analyze the `flask` package (~18 files).
#[test]
fn test_corpus_flask() {
    let Some(dir) = corpus_dir("flask", "src/flask") else {
        eprintln!("SKIP test_corpus_flask: benchmarks/corpora/flask/src/flask not found");
        return;
    };

    let (_, stats) = analyze_corpus(&dir);

    // flask has 18 source files, several classes (Flask, Blueprint, etc.)
    assert_corpus_healthy("flask", &stats, 8, 5, 20, 15);
}
