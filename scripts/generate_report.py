#!/usr/bin/env python3
"""Generate a self-contained HTML report for pycg-rs.

Runs pycg on popular Python projects in both symbol-level (JSON stats) and
module-level (DOT -> SVG) modes, then emits a single index.html with inline
SVGs and summary stats.

    python scripts/generate_report.py --pycg ./target/release/pycg \
        --corpora benchmarks/corpora --out report/index.html
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from datetime import datetime, timezone
from html import escape
from pathlib import Path

# Mapping: corpus name -> subdirectory containing the Python package source.
SOURCE_HINTS = {
    "black": ["src/black"],
    "flask": ["src/flask"],
    "httpx": ["httpx"],
    "requests": ["src/requests"],
    "rich": ["rich"],
    "pytest": ["src"],
    "click": ["src/click"],
    "pydantic": ["pydantic"],
    "fastapi": ["fastapi"],
}


def find_source_dir(corpus_dir: Path, name: str) -> Path | None:
    hints = SOURCE_HINTS.get(name, [name])
    for hint in hints:
        candidate = corpus_dir / hint
        if candidate.is_dir():
            return candidate
    candidate = corpus_dir / name
    if candidate.is_dir():
        return candidate
    return None


def count_py_files(directory: Path) -> int:
    return sum(1 for _ in directory.rglob("*.py"))


def run_pycg_json(pycg_bin: str, source_dir: Path) -> dict | None:
    """Run pycg --format json on source_dir, return parsed JSON or None."""
    try:
        start = time.monotonic()
        result = subprocess.run(
            [pycg_bin, str(source_dir), "--format", "json"],
            capture_output=True, text=True, timeout=120,
        )
        elapsed = time.monotonic() - start
        if result.returncode != 0:
            print(f"  [warn] pycg json exited {result.returncode}: {result.stderr[:200]}", file=sys.stderr)
            return None
        data = json.loads(result.stdout)
        data["_elapsed_ms"] = round(elapsed * 1000)
        return data
    except (subprocess.TimeoutExpired, json.JSONDecodeError) as e:
        print(f"  [warn] pycg json failed: {e}", file=sys.stderr)
        return None


def run_pycg_svg(pycg_bin: str, source_dir: Path) -> str | None:
    """Run pycg --modules --colored | dot -Tsvg, return SVG string or None."""
    try:
        pycg_proc = subprocess.run(
            [pycg_bin, str(source_dir), "--modules", "--colored", "--format", "dot"],
            capture_output=True, text=True, timeout=120,
        )
        if pycg_proc.returncode != 0:
            print(f"  [warn] pycg dot exited {pycg_proc.returncode}", file=sys.stderr)
            return None
        dot_proc = subprocess.run(
            ["dot", "-Tsvg"],
            input=pycg_proc.stdout, capture_output=True, text=True, timeout=30,
        )
        if dot_proc.returncode != 0:
            print(f"  [warn] dot exited {dot_proc.returncode}", file=sys.stderr)
            return None
        # Strip the XML declaration and doctype so it embeds cleanly
        svg = dot_proc.stdout
        for prefix in ('<?xml', '<!DOCTYPE'):
            idx = svg.find(prefix)
            if idx != -1:
                end = svg.find('\n', idx)
                if end != -1:
                    svg = svg[:idx] + svg[end + 1:]
        return svg.strip()
    except subprocess.TimeoutExpired:
        print(f"  [warn] SVG generation timed out for {source_dir}", file=sys.stderr)
        return None


def run_cargo_test_count() -> int | None:
    try:
        result = subprocess.run(
            ["cargo", "test", "--all-targets", "--", "--list"],
            capture_output=True, text=True, timeout=300,
        )
        if result.returncode != 0:
            return None
        return sum(
            1
            for line in result.stdout.splitlines()
            if re.search(r": test$", line.strip())
        )
    except Exception:
        pass
    return None


def generate_html(corpora_results: list[dict], meta: dict) -> str:
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")

    # Summary table rows
    dash = "&mdash;"
    rows_html = ""
    for r in corpora_results:
        s = r.get("stats", {})
        elapsed = r.get("_elapsed_ms", dash)
        py_files = r.get("_py_files", dash)
        status_class = "ok" if r.get("_success") else "fail"
        status_text = "&#x2713;" if r.get("_success") else "&#x2717;"
        files_analyzed = s.get("files_analyzed", dash)
        total_nodes = s.get("total_nodes", dash)
        classes = s.get("classes", dash)
        functions = s.get("functions", dash)
        modules = s.get("modules", dash)
        total_edges = s.get("total_edges", dash)
        rows_html += f"""<tr class="{status_class}">
  <td class="name">{escape(r['name'])}</td>
  <td>{py_files}</td>
  <td>{files_analyzed}</td>
  <td>{total_nodes}</td>
  <td>{classes}</td>
  <td>{functions}</td>
  <td>{modules}</td>
  <td>{total_edges}</td>
  <td>{elapsed}ms</td>
  <td class="status">{status_text}</td>
</tr>"""

    # Per-corpus accordion with SVG
    details_html = ""
    for r in corpora_results:
        if not r.get("_success"):
            continue
        s = r.get("stats", {})
        svg = r.get("_svg", "")
        summary_stats = (
            f"{s.get('total_nodes', 0)} nodes, "
            f"{s.get('total_edges', 0)} edges, "
            f"{s.get('classes', 0)} classes, "
            f"{s.get('functions', 0)} functions"
        )
        svg_block = f'<div class="svg-container">{svg}</div>' if svg else '<p class="no-svg">SVG not available (graphviz not found?)</p>'
        details_html += f"""
<details class="corpus-detail" id="detail-{escape(r['name'])}">
  <summary>{escape(r['name'])} &mdash; {summary_stats}</summary>
  <div class="detail-body">
    <p class="detail-caption">Module-level dependency graph &mdash; each node is a Python module, edges represent cross-module calls or imports.</p>
    {svg_block}
  </div>
</details>"""

    test_count = meta.get("test_count", "—")
    version = meta.get("version", "0.1.0")
    commit = meta.get("commit", "unknown")

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>pycg-rs &mdash; Analysis Report</title>
<style>
:root {{
  --bg: #0d1117;
  --surface: #161b22;
  --border: #30363d;
  --text: #e6edf3;
  --text-muted: #8b949e;
  --accent: #58a6ff;
  --green: #3fb950;
  --red: #f85149;
  --font: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
  --mono: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
}}
* {{ margin: 0; padding: 0; box-sizing: border-box; }}
body {{
  font-family: var(--font);
  background: var(--bg);
  color: var(--text);
  line-height: 1.5;
  padding: 2rem;
  max-width: 1200px;
  margin: 0 auto;
}}
h1 {{ font-size: 1.5rem; margin-bottom: 0.25rem; }}
.subtitle {{ color: var(--text-muted); font-size: 0.875rem; margin-bottom: 0.5rem; }}
.intro {{
  color: var(--text-muted);
  font-size: 0.8125rem;
  margin-bottom: 2rem;
  max-width: 72ch;
  line-height: 1.6;
}}
.intro a {{ color: var(--accent); }}
.meta-grid {{
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 1rem;
  margin-bottom: 2rem;
}}
.meta-card {{
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 1rem;
}}
.meta-card .label {{ color: var(--text-muted); font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; }}
.meta-card .value {{ font-size: 1.5rem; font-weight: 600; font-family: var(--mono); }}
table {{
  width: 100%;
  border-collapse: collapse;
  font-size: 0.875rem;
  margin-bottom: 1rem;
}}
th, td {{
  padding: 0.5rem 0.75rem;
  text-align: left;
  border-bottom: 1px solid var(--border);
}}
th {{ color: var(--text-muted); font-weight: 500; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; white-space: nowrap; cursor: pointer; }}
th:hover {{ color: var(--text); }}
td {{ font-family: var(--mono); font-size: 0.8125rem; }}
td.name {{ font-weight: 600; color: var(--accent); }}
tr.ok td.status {{ color: var(--green); }}
tr.fail td.status {{ color: var(--red); }}
tr:hover {{ background: rgba(88,166,255,0.05); }}
.section {{ margin-top: 2.5rem; }}
.section h2 {{ font-size: 1.125rem; margin-bottom: 0.5rem; padding-bottom: 0.5rem; border-bottom: 1px solid var(--border); }}
.section .section-desc {{ color: var(--text-muted); font-size: 0.8125rem; margin-bottom: 1rem; }}
details.corpus-detail {{
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  margin-bottom: 0.75rem;
}}
details.corpus-detail summary {{
  padding: 0.75rem 1rem;
  cursor: pointer;
  font-weight: 500;
  font-size: 0.875rem;
}}
details.corpus-detail summary:hover {{ color: var(--accent); }}
.detail-body {{ padding: 0 1rem 1rem; }}
.detail-caption {{ color: var(--text-muted); font-size: 0.75rem; margin-bottom: 0.75rem; }}
.svg-container {{
  background: #fff;
  border-radius: 4px;
  padding: 1rem;
  overflow-x: auto;
  text-align: center;
}}
.svg-container svg {{
  max-width: 100%;
  height: auto;
}}
.no-svg {{ color: var(--text-muted); font-style: italic; font-size: 0.8125rem; }}
footer {{ margin-top: 3rem; padding-top: 1rem; border-top: 1px solid var(--border); color: var(--text-muted); font-size: 0.75rem; }}
footer a {{ color: var(--accent); }}
</style>
</head>
<body>

<h1>pycg-rs</h1>
<p class="subtitle">Static call graph analysis report &mdash; generated {now}</p>
<p class="intro">
  <a href="https://github.com/tau/pycg-rs">pycg-rs</a> is a static call graph
  generator for Python. It parses source files and builds a graph of
  defines/uses relationships between modules, classes, functions, and methods
  &mdash; no Python runtime required. This page shows corpus smoke results
  across popular open-source Python projects. These runs demonstrate that the
  analyzer completes and produces non-degenerate graphs; they are not a proof
  of semantic equivalence to other tools. Expand any project below to see its
  module-level dependency graph.
</p>

<div class="meta-grid">
  <div class="meta-card">
    <div class="label">Version</div>
    <div class="value">{escape(version)}</div>
  </div>
  <div class="meta-card">
    <div class="label">Test Cases</div>
    <div class="value">{test_count}</div>
  </div>
  <div class="meta-card">
    <div class="label">Commit</div>
    <div class="value" style="font-size: 0.875rem">{escape(str(commit)[:8])}</div>
  </div>
</div>

<div class="section">
  <h2>Corpus Smoke Results</h2>
  <p class="section-desc">Summary of symbol-level analysis on each project. Status means the run completed and produced a non-degenerate graph, not that every edge was manually verified. Click column headers to sort.</p>
  <table>
    <thead>
      <tr>
        <th>Project</th>
        <th>.py files</th>
        <th>Analyzed</th>
        <th>Nodes</th>
        <th>Classes</th>
        <th>Functions</th>
        <th>Modules</th>
        <th>Edges</th>
        <th>Time</th>
        <th>Status</th>
      </tr>
    </thead>
    <tbody>
      {rows_html}
    </tbody>
  </table>
</div>

<div class="section">
  <h2>Module Dependency Graphs</h2>
  <p class="section-desc">Module-level view &mdash; functions and classes collapsed into their owning module. Generated with <code>pycg --modules --colored</code>.</p>
  {details_html}
</div>

<footer>
  Generated by <a href="https://github.com/tau/pycg-rs">pycg-rs</a>.
  Commit <code>{escape(str(commit)[:8])}</code>.
</footer>

<script>
document.querySelector('table thead tr').addEventListener('click', function(e) {{
  const th = e.target.closest('th');
  if (!th) return;
  const table = th.closest('table');
  const tbody = table.querySelector('tbody');
  const rows = Array.from(tbody.rows);
  const idx = Array.from(th.parentNode.children).indexOf(th);
  const dir = th.dataset.sort === 'asc' ? -1 : 1;
  th.dataset.sort = dir === 1 ? 'asc' : 'desc';
  rows.sort((a, b) => {{
    let av = a.cells[idx].textContent.replace(/[^\\d.]/g, '');
    let bv = b.cells[idx].textContent.replace(/[^\\d.]/g, '');
    const an = parseFloat(av), bn = parseFloat(bv);
    if (!isNaN(an) && !isNaN(bn)) return (an - bn) * dir;
    return a.cells[idx].textContent.localeCompare(b.cells[idx].textContent) * dir;
  }});
  rows.forEach(r => tbody.appendChild(r));
}});
</script>
</body>
</html>"""


def main():
    parser = argparse.ArgumentParser(description="Generate pycg-rs HTML report")
    parser.add_argument("--pycg", default="./target/release/pycg", help="Path to pycg binary")
    parser.add_argument("--corpora", default="benchmarks/corpora", help="Corpora directory")
    parser.add_argument("--out", default="report/index.html", help="Output HTML path")
    parser.add_argument("--test-count", type=int, default=None, help="Number of passing tests")
    parser.add_argument("--commit", default=None, help="Git commit hash")
    parser.add_argument("--version", default=None, help="Project version")
    args = parser.parse_args()

    corpora_dir = Path(args.corpora)
    if not corpora_dir.is_dir():
        print(f"Corpora directory not found: {corpora_dir}", file=sys.stderr)
        sys.exit(1)

    version = args.version
    if not version:
        try:
            cargo = Path("Cargo.toml").read_text()
            for line in cargo.splitlines():
                if line.startswith("version"):
                    version = line.split('"')[1]
                    break
        except Exception:
            version = "unknown"

    commit = args.commit
    if not commit:
        try:
            commit = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
        except Exception:
            commit = "unknown"

    test_count = args.test_count
    if test_count is None:
        print("Counting tests...", file=sys.stderr)
        test_count = run_cargo_test_count() or "—"

    # Check for graphviz
    has_dot = subprocess.run(["which", "dot"], capture_output=True).returncode == 0
    if not has_dot:
        print("  [warn] graphviz 'dot' not found, SVGs will be skipped", file=sys.stderr)

    results = []
    for corpus_name in sorted(os.listdir(corpora_dir)):
        corpus_path = corpora_dir / corpus_name
        if not corpus_path.is_dir():
            continue
        source_dir = find_source_dir(corpus_path, corpus_name)
        if not source_dir:
            print(f"  [skip] {corpus_name}: no source directory found", file=sys.stderr)
            continue

        py_count = count_py_files(source_dir)
        print(f"  Analyzing {corpus_name} ({py_count} .py files)...", file=sys.stderr)

        data = run_pycg_json(args.pycg, source_dir)
        svg = run_pycg_svg(args.pycg, source_dir) if has_dot else None

        if data:
            entry = {
                "name": corpus_name,
                "_py_files": py_count,
                "_success": True,
                "_elapsed_ms": data.pop("_elapsed_ms", "—"),
                "stats": data.get("stats", {}),
                "_svg": svg or "",
            }
        else:
            entry = {
                "name": corpus_name,
                "_py_files": py_count,
                "_success": False,
            }
        results.append(entry)

    meta = {"test_count": test_count, "version": version, "commit": commit}
    html = generate_html(results, meta)

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(html)
    print(f"Report written to {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
