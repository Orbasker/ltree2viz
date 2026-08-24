use crate::core::tree::{Node, Tree};

#[derive(Debug, Default)]
pub struct Options {
    /// Shown in the page title bar and as the label of the virtual root that
    /// holds every real root.
    pub title: Option<String>,
}

/// Renders the tree as a single self-contained HTML page with an interactive,
/// collapsible D3 tree.
///
/// Every real root hangs off one virtual root so a forest still renders as a
/// single collapsible tree. Size limits are intentionally not applied by the
/// caller for this format: collapsing is the mechanism for taming large trees,
/// so the full hierarchy is embedded and the reader expands what they need.
pub fn render(tree: &Tree, options: &Options) -> String {
    let root_label = options.title.clone().unwrap_or_else(|| "root".to_owned());

    let mut data = String::new();
    write_root(&mut data, &root_label, &tree.roots);

    let title = html_escape(&root_label);
    TEMPLATE
        .replace("{{TITLE}}", &title)
        .replace("{{DATA}}", &data)
}

/// Serializes the virtual root and its real-root children as one JSON object.
fn write_root(out: &mut String, label: &str, roots: &[Node]) {
    out.push_str("{\"name\":");
    write_json_string(out, label);
    out.push_str(",\"virtual\":true,\"children\":[");
    for (i, root) in roots.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_node(out, root);
    }
    out.push_str("]}");
}

fn write_node(out: &mut String, node: &Node) {
    out.push_str("{\"name\":");
    write_json_string(out, &node.label);
    if node.synthesized || node.fold.is_some() {
        out.push_str(",\"inferred\":true");
    }
    if !node.children.is_empty() {
        out.push_str(",\"children\":[");
        for (i, child) in node.children.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write_node(out, child);
        }
        out.push(']');
    }
    out.push('}');
}

/// Writes a JSON string literal, escaping so it is safe both as JSON and when
/// embedded inside an inline `<script>` (`<`, `>`, `&` and the line separators
/// are escaped so nothing can close the tag or break the parser).
fn write_json_string(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Escapes text placed in HTML element content (the `<title>`).
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

const TEMPLATE: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{{TITLE}}</title>
<style>
  :root { color-scheme: dark; }
  html, body { margin: 0; height: 100%; background: #1e1e1e; color: #e6e6e6;
    font: 13px/1.4 -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }
  #toolbar { position: fixed; top: 0; left: 0; right: 0; height: 44px; display: flex;
    align-items: center; gap: 8px; padding: 0 12px; background: #252526;
    border-bottom: 1px solid #3a3a3a; z-index: 10; box-sizing: border-box; }
  #toolbar strong { font-weight: 600; margin-right: 8px; }
  button, select { background: #333; color: #e6e6e6; border: 1px solid #4a4a4a;
    border-radius: 5px; padding: 5px 10px; font-size: 12px; cursor: pointer; }
  button:hover, select:hover { background: #3d3d3d; }
  #hint { margin-left: auto; color: #888; font-size: 11px; }
  svg { position: fixed; top: 44px; left: 0; width: 100vw; height: calc(100vh - 44px); }
  .link { fill: none; stroke: #555; stroke-width: 1.5px; }
  .node circle { stroke-width: 1.5px; cursor: pointer; }
  .node text { fill: #e6e6e6; font-size: 12px; paint-order: stroke;
    stroke: #1e1e1e; stroke-width: 3px; }
  .node--real circle { fill: #4a9eff; stroke: #7cb8ff; }
  .node--collapsed circle { fill: #2d6db5; stroke: #4a9eff; }
  .node--inferred circle { fill: #555; stroke: #888; stroke-dasharray: 2 2; }
  .node--root circle { fill: #35c46a; stroke: #86e6ab; }
  .node--leaf circle { fill: #f0a13a; stroke: #ffcc80; }
  #legend { display: flex; align-items: center; gap: 14px; margin-left: 16px; color: #bbb; font-size: 11px; }
  #legend span { display: inline-flex; align-items: center; gap: 5px; }
  #legend i { width: 10px; height: 10px; border-radius: 50%; display: inline-block; }
  #search { display: inline-flex; align-items: center; gap: 6px; }
  #search input { background: #333; color: #e6e6e6; border: 1px solid #4a4a4a;
    border-radius: 5px; padding: 5px 8px; font-size: 12px; width: 160px; }
  #search input:focus { outline: none; border-color: #4a9eff; }
  #count { color: #888; font-size: 11px; min-width: 54px; }
  .node--match circle { fill: #ffd24a; stroke: #fff3c4; }
  .node--current circle { stroke: #ffffff; stroke-width: 3px; }
  .node--match text { fill: #ffe9a8; }
  .node--dim { opacity: 0.22; }
  .link--dim { opacity: 0.12; }
</style>
</head>
<body>
<div id="toolbar">
  <strong>{{TITLE}}</strong>
  <button id="expand">Expand all</button>
  <button id="collapse">Collapse all</button>
  <label>To level
    <select id="level"></select>
  </label>
  <label>Orient
    <select id="orient">
      <option value="LR">Left → Right</option>
      <option value="RL">Right → Left</option>
      <option value="TB">Top → Bottom</option>
      <option value="BT">Bottom → Top</option>
    </select>
  </label>
  <button id="fit">Fit</button>
  <span id="search">
    <input id="q" type="search" placeholder="Search nodes… (/)" autocomplete="off" spellcheck="false">
    <button id="prev" title="Previous match (Shift+Enter)">‹</button>
    <button id="next" title="Next match (Enter)">›</button>
    <span id="count"></span>
  </span>
  <span id="legend">
    <span><i style="background:#35c46a"></i>root</span>
    <span><i style="background:#4a9eff"></i>branch</span>
    <span><i style="background:#2d6db5"></i>collapsed</span>
    <span><i style="background:#f0a13a"></i>leaf</span>
  </span>
  <span id="hint">Click a node to expand/collapse · scroll to zoom · drag to pan · / to search</span>
</div>
<svg></svg>
<script src="https://d3js.org/d3.v7.min.js"></script>
<script>
const DATA = {{DATA}};

const svg = d3.select("svg");
const g = svg.append("g");
const gLink = g.append("g");
const gNode = g.append("g");

// Sibling / level gaps differ per axis: horizontal layouts stack siblings
// tightly and spread levels wide; vertical layouts need the reverse so labels
// (which run horizontally) do not collide.
const GAP_H = [24, 220];   // [between siblings, between levels] when horizontal
const GAP_V = [140, 90];   // ...when vertical
let orient = "LR";         // LR, RL, TB, BT

const tree = d3.tree();
const horizontal = () => orient === "LR" || orient === "RL";
function applyNodeSize() { tree.nodeSize(horizontal() ? GAP_H : GAP_V); }

// Map d3's layout coordinates (d.x across siblings, d.y down the levels) onto
// screen coordinates for the chosen orientation.
const px = d => horizontal() ? (orient === "RL" ? -d.y : d.y) : d.x;
const py = d => horizontal() ? d.x : (orient === "BT" ? -d.y : d.y);
function linkPath(s, t) {
  const gen = horizontal() ? d3.linkHorizontal() : d3.linkVertical();
  return gen.x(px).y(py)({source: s, target: t});
}
function placeText(sel) {
  if (horizontal()) {
    sel.attr("dy", "0.31em")
      .attr("x", d => (d._children || d.children) ? -9 : 9)
      .attr("text-anchor", d => (d._children || d.children) ? "end" : "start");
  } else {
    // Label internal nodes on the parent side, leaves on the far side.
    const near = orient === "TB" ? "-0.9em" : "1.5em";
    const far = orient === "TB" ? "1.5em" : "-0.9em";
    sel.attr("x", 0).attr("text-anchor", "middle")
      .attr("dy", d => (d._children || d.children) ? near : far);
  }
}

const root = d3.hierarchy(DATA);
root.x0 = 0;
root.y0 = 0;
let maxDepth = 0;
root.descendants().forEach((d, i) => {
  d.id = i;
  d._children = d.children;
  maxDepth = Math.max(maxDepth, d.depth);
});

const zoom = d3.zoom().scaleExtent([0.05, 3]).on("zoom", e => g.attr("transform", e.transform));
svg.call(zoom);

function collapse(d) { if (d.children) { d.children.forEach(collapse); d.children = null; } }
function expandAll(d) { if (d._children) d.children = d._children; if (d.children) d.children.forEach(expandAll); }
function collapseToDepth(d, depth) {
  if (d.depth < depth) { d.children = d._children; }
  else { d.children = null; }
  (d._children || []).forEach(c => collapseToDepth(c, depth));
}

// Search state. `matches` holds matched nodes from the full hierarchy (in
// pre-order); `savedState` snapshots the collapse layout so clearing restores it.
let matches = [];
let matchIndex = -1;
let savedState = null;

function walkAll(fn) { (function rec(d) { fn(d); if (d._children) d._children.forEach(rec); })(root); }

function update(source) {
  applyNodeSize();
  const nodes = root.descendants();
  const links = root.links();
  tree(root);

  const t = svg.transition().duration(250);
  const origin0 = {x: source.x0, y: source.y0};
  const origin = {x: source.x, y: source.y};

  const node = gNode.selectAll("g.node").data(nodes, d => d.id);
  const nodeEnter = node.enter().append("g")
    .attr("class", "node")
    .attr("transform", `translate(${px(origin0)},${py(origin0)})`)
    .attr("fill-opacity", 0).attr("stroke-opacity", 0)
    .on("click", (event, d) => {
      d.children = d.children ? null : d._children;
      update(d);
    });
  nodeEnter.append("circle").attr("r", 5);
  nodeEnter.append("text").text(d => d.data.name);

  node.merge(nodeEnter).transition(t)
    .attr("transform", d => `translate(${px(d)},${py(d)})`)
    .attr("fill-opacity", 1).attr("stroke-opacity", 1)
    .attr("class", d => {
      let c = "node";
      if (d.depth === 0) c += " node--root";
      else if (d.data.inferred) c += " node--inferred";
      else if (!d._children) c += " node--leaf";
      else if (!d.children) c += " node--collapsed";
      else c += " node--real";
      return c;
    });
  node.merge(nodeEnter).select("text").call(placeText);

  node.exit().transition(t).remove()
    .attr("transform", `translate(${px(origin)},${py(origin)})`)
    .attr("fill-opacity", 0).attr("stroke-opacity", 0);

  const link = gLink.selectAll("path.link").data(links, d => d.target.id);
  const linkEnter = link.enter().append("path").attr("class", "link")
    .attr("d", () => linkPath(origin0, origin0));
  link.merge(linkEnter).transition(t).attr("d", d => linkPath(d.source, d.target));
  link.exit().transition(t).remove()
    .attr("d", () => linkPath(origin, origin));

  root.eachBefore(d => { d.x0 = d.x; d.y0 = d.y; });
  applySearchStyles();
}

// Highlights matches, dims everything else, and marks the current match.
// Re-run after every update() so click/expand redraws keep search styling.
function applySearchStyles() {
  const active = matches.length > 0;
  const current = matchIndex >= 0 ? matches[matchIndex] : null;
  gNode.selectAll("g.node")
    .classed("node--match", d => active && d._match)
    .classed("node--dim", d => active && !d._match)
    .classed("node--current", d => d === current);
  gLink.selectAll("path.link").classed("link--dim", () => active);
}

function centerOn(d) {
  const k = d3.zoomTransform(svg.node()).k;
  const fullW = window.innerWidth, fullH = window.innerHeight - 44;
  const target = d3.zoomIdentity.translate(fullW / 2 - k * px(d), fullH / 2 - k * py(d)).scale(k);
  svg.transition().duration(300).call(zoom.transform, target);
}

function updateCount() {
  const el = document.getElementById("count");
  if (matches.length) el.textContent = `${matchIndex + 1} of ${matches.length}`;
  else el.textContent = document.getElementById("q").value.trim() ? "0 matches" : "";
}

// Free search: the query is split on whitespace into tokens that must each
// appear as a case-insensitive substring, in order. This skips gaps and
// separators, so "mens ss tops" matches "MENS_SS_LS_TOPS". A single token
// degrades to a plain substring match.
function matchLabel(label, tokens) {
  const hay = label.toLowerCase();
  let from = 0;
  for (const tok of tokens) {
    const idx = hay.indexOf(tok, from);
    if (idx < 0) return false;
    from = idx + tok.length;
  }
  return true;
}

function runSearch() {
  const raw = document.getElementById("q").value.trim();
  if (!raw) { clearSearch(); return; }
  const tokens = raw.toLowerCase().split(/\s+/).filter(Boolean);
  if (!savedState) {
    savedState = new Map();
    walkAll(d => savedState.set(d.id, d.children != null));
  }
  matches = [];
  walkAll(d => {
    const hit = !d.data.virtual && matchLabel(d.data.name, tokens);
    d._match = hit;
    if (hit) matches.push(d);
  });
  // Reveal every match by expanding its ancestors; other branches keep their state.
  matches.forEach(d => { for (let p = d.parent; p; p = p.parent) p.children = p._children; });
  matchIndex = matches.length ? 0 : -1;
  update(root);
  updateCount();
  if (matchIndex >= 0) centerOn(matches[matchIndex]);
}

function clearSearch() {
  document.getElementById("q").value = "";
  matches = []; matchIndex = -1;
  walkAll(d => { d._match = false; });
  if (savedState) { walkAll(d => { d.children = savedState.get(d.id) ? d._children : null; }); savedState = null; }
  update(root);
  updateCount();
  fit();
}

function navigate(delta) {
  if (!matches.length) return;
  matchIndex = (matchIndex + delta + matches.length) % matches.length;
  applySearchStyles();
  updateCount();
  centerOn(matches[matchIndex]);
}

// Fits from the layout coordinates rather than getBBox so it is correct even
// while node/link transitions are still animating.
function fit(animate = true) {
  const nodes = root.descendants();
  if (!nodes.length) return;
  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
  nodes.forEach(d => {
    const x = px(d), y = py(d);
    if (x < minX) minX = x; if (x > maxX) maxX = x;
    if (y < minY) minY = y; if (y > maxY) maxY = y;
  });
  const pad = 40, labelPad = 200;       // room for node radius and labels
  const w = (maxX - minX) + labelPad;
  const h = (maxY - minY) + 2 * pad;
  const fullW = window.innerWidth, fullH = window.innerHeight - 44;
  const scale = Math.min(0.95 * fullW / w, 0.95 * fullH / h, 1.5);
  const cx = (minX + maxX) / 2, cy = (minY + maxY) / 2;
  const tx = fullW / 2 - scale * cx;
  const ty = fullH / 2 - scale * cy;
  const target = d3.zoomIdentity.translate(tx, ty).scale(scale);
  const sel = animate ? svg.transition().duration(300) : svg;
  sel.call(zoom.transform, target);
}

const levelSelect = document.getElementById("level");
for (let i = 1; i <= Math.max(1, maxDepth); i++) {
  const opt = document.createElement("option");
  opt.value = i; opt.textContent = i; levelSelect.appendChild(opt);
}
document.getElementById("expand").onclick = () => { expandAll(root); update(root); fit(); };
document.getElementById("collapse").onclick = () => { root.children && root.children.forEach(collapse); update(root); fit(); };
document.getElementById("fit").onclick = fit;
levelSelect.onchange = () => { collapseToDepth(root, +levelSelect.value); update(root); fit(); };
document.getElementById("orient").onchange = e => { orient = e.target.value; update(root); fit(); };

const searchInput = document.getElementById("q");
searchInput.addEventListener("input", runSearch);
searchInput.addEventListener("keydown", e => {
  if (e.key === "Enter") { e.preventDefault(); navigate(e.shiftKey ? -1 : 1); }
  else if (e.key === "Escape") { e.preventDefault(); clearSearch(); searchInput.blur(); }
});
document.getElementById("next").onclick = () => navigate(1);
document.getElementById("prev").onclick = () => navigate(-1);
document.addEventListener("keydown", e => {
  if (e.key === "/" && e.target !== searchInput) { e.preventDefault(); searchInput.focus(); }
});

// Start with the first level open, fit once the initial layout exists.
collapseToDepth(root, 1);
update(root);
fit(false);
</script>
</body>
</html>
"##;
