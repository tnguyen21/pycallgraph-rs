//! Output writers for the visual call graph.
//!
//! Provides functions to serialize a [`VisualGraph`] into DOT (GraphViz),
//! TGF (Trivial Graph Format), plain text, and JSON.

use crate::node::{Node, NodeId};
use crate::visgraph::{VisualGraph, VisualNode};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// DOT writer
// ---------------------------------------------------------------------------

/// Render the visual graph in GraphViz DOT format.
///
/// `options` is a list of extra top-level graph attributes (e.g.
/// `rankdir=LR`).  When the graph is grouped, `clusterrank="local"` is
/// appended automatically.
pub fn write_dot(graph: &VisualGraph, options: &[String]) -> String {
    let mut out = String::new();

    // Collect graph-level options.
    let mut opts: Vec<String> = options.to_vec();
    if graph.grouped {
        opts.push("clusterrank=\"local\"".to_string());
    }
    let opts_str = opts.join(", ");

    writeln!(out, "digraph G {{").unwrap();
    writeln!(out, "    graph [{opts_str}];").unwrap();

    if graph.grouped && !graph.subgraphs.is_empty() {
        for sg in &graph.subgraphs {
            write_dot_subgraph(&mut out, sg, 1);
        }
    } else {
        // No subgraphs – emit all nodes at root level.
        for node in &graph.nodes {
            write_dot_node(&mut out, node, 1);
        }
    }

    // Edges (always at root level).
    for edge in &graph.edges {
        let src = &graph.nodes[edge.source_idx];
        let tgt = &graph.nodes[edge.target_idx];
        let style = if edge.flavor == "defines" {
            "dashed"
        } else {
            "solid"
        };
        let color = &edge.color;
        writeln!(
            out,
            "    {} -> {} [style=\"{style}\", color=\"{color}\"];",
            src.id, tgt.id
        )
        .unwrap();
    }

    writeln!(out, "}}").unwrap();
    out
}

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

fn write_dot_node(out: &mut String, node: &VisualNode, level: usize) {
    let pad = indent(level);
    writeln!(
        out,
        "{pad}{id} [label=\"{label}\", style=\"filled\", fillcolor=\"{fill}\", fontcolor=\"{text}\", group=\"{group}\"];",
        id = node.id,
        label = node.label,
        fill = node.fill_color,
        text = node.text_color,
        group = node.group,
    )
    .unwrap();
}

fn write_dot_subgraph(out: &mut String, sg: &VisualGraph, level: usize) {
    let pad = indent(level);
    writeln!(out, "{pad}subgraph cluster_{id} {{", id = sg.id).unwrap();

    let inner = indent(level + 1);
    writeln!(
        out,
        "{inner}graph [style=\"filled,rounded\", fillcolor=\"#80808018\", label=\"{label}\"];",
        label = sg.label,
    )
    .unwrap();

    for node in &sg.nodes {
        write_dot_node(out, node, level + 1);
    }

    for child in &sg.subgraphs {
        write_dot_subgraph(out, child, level + 1);
    }

    writeln!(out, "{pad}}}").unwrap();
}

// ---------------------------------------------------------------------------
// TGF writer
// ---------------------------------------------------------------------------

/// Render the visual graph in Trivial Graph Format.
///
/// Nodes are numbered sequentially starting at 1.
pub fn write_tgf(graph: &VisualGraph) -> String {
    let mut out = String::new();

    // Assign sequential 1-based IDs.
    for (i, node) in graph.nodes.iter().enumerate() {
        writeln!(out, "{} {}", i + 1, node.label).unwrap();
    }

    writeln!(out, "#").unwrap();

    for edge in &graph.edges {
        let tag = if edge.flavor == "uses" { "U" } else { "D" };
        writeln!(out, "{} {} {tag}", edge.source_idx + 1, edge.target_idx + 1).unwrap();
    }

    out
}

// ---------------------------------------------------------------------------
// Text writer
// ---------------------------------------------------------------------------

/// Render the visual graph as a plain-text dependency list.
///
/// Each source node is printed on its own line, followed by its outgoing
/// edges indented with `[D]` (defines) or `[U]` (uses) tags.  Output is
/// sorted alphabetically by source label, then by (tag, target label).
pub fn write_text(graph: &VisualGraph) -> String {
    use std::collections::BTreeMap;

    // Build adjacency: source label → sorted Vec<(tag, target label)>.
    let mut adj: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();

    for edge in &graph.edges {
        let src_label = graph.nodes[edge.source_idx].label.as_str();
        let tgt_label = graph.nodes[edge.target_idx].label.as_str();
        let tag = if edge.flavor == "defines" { "D" } else { "U" };
        adj.entry(src_label).or_default().push((tag, tgt_label));
    }

    let mut out = String::new();
    for (src, targets) in &mut adj {
        targets.sort();
        writeln!(out, "{src}").unwrap();
        for (tag, tgt) in targets {
            writeln!(out, "    [{tag}] {tgt}").unwrap();
        }
    }

    out
}

// ---------------------------------------------------------------------------
// JSON writer
// ---------------------------------------------------------------------------

pub enum JsonGraphMode {
    Symbol,
    Module,
}

pub struct JsonOutputOptions<'a> {
    pub graph_mode: JsonGraphMode,
    pub analysis_root: Option<&'a str>,
    pub inputs: &'a [String],
}

struct PathFormatter {
    root: Option<PathBuf>,
    cwd: PathBuf,
    path_kind: &'static str,
}

impl PathFormatter {
    fn new(root: Option<&str>, inputs: &[String]) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let root = root.map(|value| Self::resolve_path(&cwd, value));
        let path_kind = if root.is_some() {
            "root_relative"
        } else if inputs.iter().all(|value| !Path::new(value).is_absolute()) {
            "input_relative"
        } else {
            "absolute"
        };
        Self {
            root,
            cwd,
            path_kind,
        }
    }

    fn resolve_path(base: &Path, path: &str) -> PathBuf {
        let path = Path::new(path);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            base.join(path)
        }
    }

    fn format_analysis_root(&self, root: &str) -> String {
        let path = Self::resolve_path(&self.cwd, root);
        self.display_path(&path)
    }

    fn format_input(&self, input: &str) -> String {
        let path = Self::resolve_path(&self.cwd, input);
        self.format_graph_path(&path)
    }

    fn format_location(&self, path: &str) -> String {
        self.format_graph_path(&Self::resolve_path(&self.cwd, path))
    }

    fn format_graph_path(&self, path: &Path) -> String {
        if let Some(root) = &self.root
            && let Ok(relative) = path.strip_prefix(root)
        {
            return Self::path_to_string(relative);
        }

        match self.path_kind {
            "input_relative" => self.display_path(path),
            _ => Self::path_to_string(path),
        }
    }

    fn display_path(&self, path: &Path) -> String {
        if let Ok(relative) = path.strip_prefix(&self.cwd) {
            if relative.as_os_str().is_empty() {
                ".".to_string()
            } else {
                Self::path_to_string(relative)
            }
        } else {
            Self::path_to_string(path)
        }
    }

    fn path_to_string(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }
}

#[derive(Serialize)]
struct JsonTool {
    name: &'static str,
    version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit: Option<&'static str>,
}

#[derive(Serialize)]
struct JsonAnalysis {
    #[serde(skip_serializing_if = "Option::is_none")]
    root: Option<String>,
    inputs: Vec<String>,
    node_inclusion_policy: &'static str,
    path_kind: &'static str,
}

#[derive(Serialize)]
struct JsonStats {
    nodes: usize,
    edges: usize,
    files_analyzed: usize,
    by_node_kind: BTreeMap<String, usize>,
    by_edge_kind: BTreeMap<String, usize>,
}

#[derive(Serialize)]
struct JsonLocation {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
}

#[derive(Serialize)]
struct JsonNode {
    id: String,
    kind: String,
    canonical_name: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<JsonLocation>,
}

#[derive(Serialize)]
struct JsonEdge {
    kind: &'static str,
    source: String,
    target: String,
}

#[derive(Serialize)]
struct JsonDiagnosticSummary {
    warnings: usize,
    unresolved_references: usize,
    ambiguous_resolutions: usize,
    external_references: usize,
    approximations: usize,
}

#[derive(Serialize)]
struct JsonWarning {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
}

#[derive(Serialize)]
struct JsonUnresolvedReference {
    kind: String,
    source: String,
    symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
}

#[derive(Serialize)]
struct JsonAmbiguousResolution {
    kind: String,
    source: String,
    symbol: String,
    candidate_targets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
}

#[derive(Serialize)]
struct JsonExternalReference {
    kind: String,
    source: String,
    canonical_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
}

#[derive(Serialize)]
struct JsonApproximation {
    kind: String,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
    reason: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    candidate_targets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
}

#[derive(Serialize)]
struct JsonDiagnostics {
    summary: JsonDiagnosticSummary,
    warnings: Vec<JsonWarning>,
    unresolved_references: Vec<JsonUnresolvedReference>,
    ambiguous_resolutions: Vec<JsonAmbiguousResolution>,
    external_references: Vec<JsonExternalReference>,
    approximations: Vec<JsonApproximation>,
}

#[derive(Serialize)]
struct JsonGraph {
    schema_version: &'static str,
    tool: JsonTool,
    graph_mode: &'static str,
    analysis: JsonAnalysis,
    stats: JsonStats,
    nodes: Vec<JsonNode>,
    edges: Vec<JsonEdge>,
    diagnostics: JsonDiagnostics,
}

fn graph_mode_label(mode: &JsonGraphMode) -> &'static str {
    match mode {
        JsonGraphMode::Symbol => "symbol",
        JsonGraphMode::Module => "module",
    }
}

fn public_kind(node: &Node) -> Option<String> {
    use crate::node::Flavor;

    match node.flavor {
        Flavor::Module => Some("module".to_string()),
        Flavor::Class => Some("class".to_string()),
        Flavor::Function => Some("function".to_string()),
        Flavor::Method => Some("method".to_string()),
        Flavor::StaticMethod => Some("static_method".to_string()),
        Flavor::ClassMethod => Some("class_method".to_string()),
        _ => None,
    }
}

fn node_name_and_namespace(node: &Node, canonical_name: &str) -> (String, Option<String>) {
    if let Some(namespace) = node.namespace.as_ref().filter(|value| !value.is_empty()) {
        return (node.name.clone(), Some(namespace.clone()));
    }

    if let Some((namespace, name)) = canonical_name.rsplit_once('.') {
        return (name.to_string(), Some(namespace.to_string()));
    }

    (canonical_name.to_string(), None)
}

/// Render the call graph directly as JSON.
///
/// Unlike the other writers which operate on the visual graph, this serializes
/// the raw call graph data for machine consumption.
pub fn write_json(
    nodes_arena: &[Node],
    defined: &HashSet<NodeId>,
    defines_edges: &HashMap<NodeId, HashSet<NodeId>>,
    uses_edges: &HashMap<NodeId, HashSet<NodeId>>,
    options: &JsonOutputOptions<'_>,
) -> String {
    let path_formatter = PathFormatter::new(options.analysis_root, options.inputs);
    let mut nodes = Vec::new();
    let mut sorted_ids: Vec<NodeId> = defined.iter().copied().collect();
    sorted_ids.sort_by(|&a, &b| {
        let na = &nodes_arena[a];
        let nb = &nodes_arena[b];
        (&na.namespace, &na.name).cmp(&(&nb.namespace, &nb.name))
    });

    let mut files: HashSet<&str> = HashSet::new();
    let mut node_kind_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut node_ids = HashMap::new();

    for (index, &id) in sorted_ids.iter().enumerate() {
        node_ids.insert(id, format!("n{}", index + 1));
    }

    for &id in &sorted_ids {
        let n = &nodes_arena[id];
        let canonical_name = n.get_name();
        let (name, namespace) = node_name_and_namespace(n, &canonical_name);
        if let Some(ref f) = n.filename {
            files.insert(f.as_str());
        }
        let kind = public_kind(n).unwrap_or_else(|| "unknown".to_string());
        *node_kind_counts.entry(kind.clone()).or_insert(0) += 1;
        nodes.push(JsonNode {
            id: node_ids
                .get(&id)
                .expect("sorted node should have an assigned id")
                .clone(),
            kind,
            canonical_name,
            name,
            namespace,
            location: n.filename.as_ref().map(|filename| JsonLocation {
                path: path_formatter.format_location(filename),
                line: n.line,
            }),
        });
    }

    let defined_set: &HashSet<NodeId> = defined;
    let mut edges = Vec::new();
    let mut edge_kind_counts: BTreeMap<String, usize> = BTreeMap::new();

    for (&src, targets) in defines_edges {
        if !defined_set.contains(&src) {
            continue;
        }
        for &tgt in targets {
            if !defined_set.contains(&tgt) {
                continue;
            }
            edges.push(JsonEdge {
                kind: "defines",
                source: node_ids
                    .get(&src)
                    .expect("defined source node should have an assigned id")
                    .clone(),
                target: node_ids
                    .get(&tgt)
                    .expect("defined target node should have an assigned id")
                    .clone(),
            });
            *edge_kind_counts.entry("defines".to_string()).or_insert(0) += 1;
        }
    }

    for (&src, targets) in uses_edges {
        if !defined_set.contains(&src) {
            continue;
        }
        for &tgt in targets {
            if !defined_set.contains(&tgt) {
                continue;
            }
            edges.push(JsonEdge {
                kind: "uses",
                source: node_ids
                    .get(&src)
                    .expect("defined source node should have an assigned id")
                    .clone(),
                target: node_ids
                    .get(&tgt)
                    .expect("defined target node should have an assigned id")
                    .clone(),
            });
            *edge_kind_counts.entry("uses".to_string()).or_insert(0) += 1;
        }
    }

    edges.sort_by(|a, b| (&a.source, &a.target, a.kind).cmp(&(&b.source, &b.target, b.kind)));

    let warnings: Vec<JsonWarning> = Vec::new();
    let unresolved_references: Vec<JsonUnresolvedReference> = Vec::new();
    let ambiguous_resolutions: Vec<JsonAmbiguousResolution> = Vec::new();
    let external_references: Vec<JsonExternalReference> = Vec::new();
    let approximations: Vec<JsonApproximation> = Vec::new();

    let graph = JsonGraph {
        schema_version: "1",
        tool: JsonTool {
            name: env!("CARGO_PKG_NAME"),
            version: env!("CARGO_PKG_VERSION"),
            commit: option_env!("PYCG_RS_GIT_COMMIT"),
        },
        graph_mode: graph_mode_label(&options.graph_mode),
        analysis: JsonAnalysis {
            root: options
                .analysis_root
                .map(|root| path_formatter.format_analysis_root(root)),
            inputs: options
                .inputs
                .iter()
                .map(|input| path_formatter.format_input(input))
                .collect(),
            node_inclusion_policy: "defined_only",
            path_kind: path_formatter.path_kind,
        },
        stats: JsonStats {
            nodes: nodes.len(),
            edges: edges.len(),
            files_analyzed: files.len(),
            by_node_kind: node_kind_counts,
            by_edge_kind: edge_kind_counts,
        },
        nodes,
        edges,
        diagnostics: JsonDiagnostics {
            summary: JsonDiagnosticSummary {
                warnings: warnings.len(),
                unresolved_references: unresolved_references.len(),
                ambiguous_resolutions: ambiguous_resolutions.len(),
                external_references: external_references.len(),
                approximations: approximations.len(),
            },
            warnings,
            unresolved_references,
            ambiguous_resolutions,
            external_references,
            approximations,
        },
    };

    serde_json::to_string_pretty(&graph).expect("JSON serialization failed")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Flavor, Node};
    use crate::visgraph::VisualOptions;
    use std::collections::{HashMap, HashSet};

    fn make_test_graph() -> VisualGraph {
        let nodes_arena = vec![
            Node::new(Some("pkg"), "Foo", Flavor::Class).with_location("pkg.py", 1),
            Node::new(Some("pkg"), "bar", Flavor::Function).with_location("pkg.py", 10),
            Node::new(Some("other"), "baz", Flavor::Function).with_location("other.py", 5),
        ];
        let mut defined = HashSet::new();
        defined.insert(0);
        defined.insert(1);
        defined.insert(2);

        let mut uses = HashMap::new();
        uses.entry(0).or_insert_with(HashSet::new).insert(1);
        uses.entry(1).or_insert_with(HashSet::new).insert(2);

        let mut defines = HashMap::new();
        defines.entry(0).or_insert_with(HashSet::new).insert(1);

        let options = VisualOptions {
            draw_defines: true,
            draw_uses: true,
            colored: true,
            grouped: false,
            annotated: false,
        };

        VisualGraph::from_call_graph(&nodes_arena, &defined, &defines, &uses, &options)
    }

    #[test]
    fn test_dot_output_structure() {
        let g = make_test_graph();
        let dot = write_dot(&g, &["rankdir=TB".to_string()]);
        assert!(dot.starts_with("digraph G {"));
        assert!(dot.contains("rankdir=TB"));
        assert!(dot.contains("style=\"filled\""));
        assert!(dot.ends_with("}\n"));
    }

    #[test]
    fn test_dot_grouped() {
        let nodes_arena = vec![
            Node::new(Some("pkg"), "A", Flavor::Class).with_location("pkg.py", 1),
            Node::new(Some("other"), "B", Flavor::Function).with_location("other.py", 5),
        ];
        let mut defined = HashSet::new();
        defined.insert(0);
        defined.insert(1);

        let options = VisualOptions {
            draw_defines: false,
            draw_uses: false,
            colored: false,
            grouped: true,
            annotated: false,
        };

        let g = VisualGraph::from_call_graph(
            &nodes_arena,
            &defined,
            &HashMap::new(),
            &HashMap::new(),
            &options,
        );
        let dot = write_dot(&g, &[]);
        assert!(dot.contains("subgraph cluster_"));
        assert!(dot.contains("clusterrank=\"local\""));
    }

    #[test]
    fn test_tgf_output() {
        let g = make_test_graph();
        let tgf = write_tgf(&g);
        // Should contain node lines, separator, and edge lines.
        assert!(tgf.contains("#\n"));
        // Nodes are 1-indexed.
        assert!(tgf.contains("1 "));
    }

    #[test]
    fn test_text_output() {
        let g = make_test_graph();
        let text = write_text(&g);
        // Should contain [U] and [D] tags.
        assert!(text.contains("[U]") || text.contains("[D]"));
    }
}
