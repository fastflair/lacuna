//! WGSL shader source and SVG + interactive WebGPU/Shader HTML spatial explorer renderers.

use std::fmt::Write;

use lacuna_core::{CorpusSnapshot, FieldGrid, VoidCandidate};
use lacuna_rung::VerifiedOpportunity;
use serde::{Deserialize, Serialize};

/// Embedded WGSL source for the continuous supply/demand/whitespace potential field shader.
pub const FIELD_SHADER_WGSL: &str = include_str!("field.wgsl");

/// Complete visual bundle ready for serialization or rendering to SVG / interactive HTML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplorerBundle {
    pub snapshot: CorpusSnapshot,
    pub entity_coords_2d: Vec<[f64; 2]>,
    pub field: FieldGrid,
    pub voids: Vec<VoidCandidate>,
    pub opportunities: Vec<VerifiedOpportunity>,
}

/// Renders a standalone vector SVG topographic map of the manifold, including:
/// - continuous potential-field heatmap tiles (`supply_density` vs `whitespace_potential`),
/// - bounded Voronoi polygons for the top ranked voids,
/// - Delaunay simplex tether lines connecting each void to its 3 boundary neighbors,
/// - corpus entity nodes and glowing cyan whitespace markers.
pub fn render_svg_manifold(bundle: &ExplorerBundle, width: usize, height: usize) -> String {
    let w = width.max(400) as f64;
    let h = height.max(400) as f64;
    let to_px = |pt: [f64; 2]| -> (f64, f64) {
        let x = (pt[0] + 1.0) * 0.5 * w;
        let y = (1.0 - (pt[1] + 1.0) * 0.5) * h;
        (x, y)
    };

    let mut svg = String::with_capacity(32_768);
    let _ = writeln!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" style="background:#080c14;font-family:monospace">"##
    );

    // 1. Render field grid cells
    let res = bundle.field.resolution.max(1);
    let cell_w = (w / res as f64) + 0.8;
    let cell_h = (h / res as f64) + 0.8;
    for sample in &bundle.field.samples {
        let (cx, cy) = to_px([sample.x, sample.y]);
        let r = (12.0 + 190.0 * sample.supply_density).round() as u8;
        let g = (18.0 + 215.0 * sample.whitespace_potential + 40.0 * sample.demand_gravity)
            .min(255.0)
            .round() as u8;
        let b = (28.0 + 230.0 * sample.whitespace_potential + 65.0 * sample.supply_density)
            .min(255.0)
            .round() as u8;
        let _ = writeln!(
            svg,
            r##"  <rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="rgb({r},{g},{b})" fill-opacity="0.72"/>"##,
            cx - cell_w * 0.5,
            cy - cell_h * 0.5,
            cell_w,
            cell_h
        );
    }

    // 2. Render Voronoi polygons & Delaunay tethers for top 5 voids
    for (idx, void) in bundle.voids.iter().take(5).enumerate() {
        if void.voronoi_polygon.len() >= 3 {
            let mut pts_str = String::new();
            for &v in &void.voronoi_polygon {
                let (px, py) = to_px(v);
                let _ = write!(pts_str, "{px:.1},{py:.1} ");
            }
            let stroke = if idx == 0 { "#00f5d4" } else { "#38bdf8" };
            let fill_op = if idx == 0 { "0.16" } else { "0.07" };
            let _ = writeln!(
                svg,
                r##"  <polygon points="{pts_str}" fill="{stroke}" fill-opacity="{fill_op}" stroke="{stroke}" stroke-width="1.5" stroke-dasharray="5,3"/>"##
            );
        }

        let (vx, vy) = to_px(void.coords_2d);
        for neighbor in &void.boundary_neighbors {
            if let Some(e_idx) = bundle
                .snapshot
                .entities
                .iter()
                .position(|e| e.id == neighbor.entity_id)
            {
                let (nx, ny) = to_px(bundle.entity_coords_2d[e_idx]);
                let _ = writeln!(
                    svg,
                    r##"  <line x1="{vx:.1}" y1="{vy:.1}" x2="{nx:.1}" y2="{ny:.1}" stroke="#00f5d4" stroke-opacity="0.45" stroke-width="1.2"/>"##
                );
            }
        }
    }

    // 3. Render corpus entities
    for (entity, &coord) in bundle
        .snapshot
        .entities
        .iter()
        .zip(bundle.entity_coords_2d.iter())
    {
        let (ex, ey) = to_px(coord);
        let rad = (3.5 + (entity.demand_signal * 0.35)).clamp(3.5, 9.0);
        let _ = writeln!(
            svg,
            r##"  <circle cx="{ex:.1}" cy="{ey:.1}" r="{rad:.1}" fill="#f43f5e" stroke="#ffe4e6" stroke-width="1"/>"##
        );
        let _ = writeln!(
            svg,
            r##"  <text x="{:.1}" y="{:.1}" fill="#e2e8f0" font-size="10">{}</text>"##,
            ex + rad + 4.0,
            ey + 3.0,
            escape_xml(&entity.name)
        );
    }

    // 4. Render void centers as glowing cyan stars
    for void in bundle.voids.iter().take(5) {
        let (vx, vy) = to_px(void.coords_2d);
        let _ = writeln!(
            svg,
            r##"  <circle cx="{vx:.1}" cy="{vy:.1}" r="7" fill="#00f5d4" stroke="#080c14" stroke-width="2"/>"##
        );
        let _ = writeln!(
            svg,
            r##"  <text x="{:.1}" y="{:.1}" fill="#00f5d4" font-weight="bold" font-size="11">{} (r={:.2})</text>"##,
            vx + 10.0,
            vy - 6.0,
            escape_xml(&void.void_id),
            void.radius
        );
    }

    svg.push_str("</svg>\n");
    svg
}

/// Renders a self-contained interactive WebGPU + Shader Canvas Explorer HTML file
/// embedding the full `ExplorerBundle` JSON, the WGSL shader (`field.wgsl`), interactive
/// supply/demand/whitespace shader sliders, clickable Voronoi cells and game nodes, and the live
/// Ghost Steam Store Page + `rung_std::decide` Bayesian viability inspector.
pub fn render_webgpu_explorer_html(bundle: &ExplorerBundle) -> Result<String, String> {
    let data_json =
        serde_json::to_string(bundle).map_err(|e| format!("serialize explorer bundle: {e}"))?;
    let wgsl_escaped = serde_json::to_string(FIELD_SHADER_WGSL)
        .map_err(|e| format!("serialize WGSL shader: {e}"))?;

    let title = escape_xml(&bundle.snapshot.schema.title);
    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>{title} — lacuna Whitespace Explorer</title>
<style>
  :root {{
    --bg: #070b12;
    --panel: #0f172a;
    --border: #1e293b;
    --cyan: #00f5d4;
    --rose: #f43f5e;
    --amber: #f59e0b;
    --text: #e2e8f0;
    --muted: #94a3b8;
  }}
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{
    background: var(--bg);
    color: var(--text);
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    display: grid;
    grid-template-columns: 1fr 460px;
    height: 100vh;
    overflow: hidden;
  }}
  @media (max-width: 980px) {{
    body {{ grid-template-columns: 1fr; grid-template-rows: 55vh 45vh; }}
  }}
  #stage {{
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
  }}
  header {{
    padding: 10px 16px;
    background: rgba(15, 23, 42, 0.95);
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
  }}
  .brand {{ display: flex; align-items: center; gap: 10px; }}
  header h1 {{ font-size: 14px; color: var(--cyan); letter-spacing: 0.04em; }}
  .pill {{
    font-size: 10px;
    padding: 2px 7px;
    border-radius: 999px;
    border: 1px solid rgba(0, 245, 212, 0.4);
    color: var(--cyan);
    background: rgba(0, 245, 212, 0.08);
    text-decoration: none;
  }}
  .controls {{ display: flex; gap: 14px; align-items: center; font-size: 11px; color: var(--muted); flex-wrap: wrap; }}
  .controls label {{ display: flex; align-items: center; gap: 5px; cursor: pointer; }}
  #canvas-wrap {{ position: relative; flex: 1; background: #05080f; overflow: hidden; }}
  canvas {{ position: absolute; inset: 0; width: 100%; height: 100%; display: block; }}
  #overlay {{ cursor: pointer; }}
  #hud-legend {{
    position: absolute;
    left: 14px;
    bottom: 14px;
    background: rgba(7, 11, 18, 0.86);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 11px;
    color: var(--muted);
    pointer-events: none;
    line-height: 1.6;
  }}
  aside {{
    background: var(--panel);
    border-left: 1px solid var(--border);
    padding: 14px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }}
  .card {{
    background: rgba(7, 11, 18, 0.78);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 12px;
  }}
  .card h2 {{ font-size: 11px; color: var(--cyan); margin-bottom: 6px; letter-spacing: 0.05em; }}
  .card h3 {{ font-size: 15px; color: #fff; margin-bottom: 6px; }}
  .badge {{
    display: inline-block;
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 11px;
    margin-right: 4px;
    margin-bottom: 4px;
    background: rgba(0, 245, 212, 0.14);
    color: var(--cyan);
    border: 1px solid rgba(0, 245, 212, 0.35);
  }}
  .void-list {{ display: flex; flex-direction: column; gap: 6px; max-height: 210px; overflow-y: auto; }}
  .void-btn {{
    text-align: left;
    background: rgba(15, 23, 42, 0.8);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 7px 10px;
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
    transition: border-color 0.15s;
  }}
  .void-btn:hover {{ border-color: #38bdf8; }}
  .void-btn.active {{ border-color: var(--cyan); background: rgba(0, 245, 212, 0.12); }}
  .meter {{ height: 6px; background: #1e293b; border-radius: 3px; overflow: hidden; margin: 3px 0 7px; }}
  .meter > div {{ height: 100%; background: var(--cyan); }}
  p {{ font-size: 12px; line-height: 1.5; color: var(--muted); }}
  pre {{ font-size: 10px; color: #cbd5e1; white-space: pre-wrap; word-break: break-word; max-height: 180px; overflow-y: auto; }}
</style>
</head>
<body>
<div id="stage">
  <header>
    <div class="brand">
      <h1>LACUNA // {title}</h1>
      <span id="gpu-badge" class="pill">Initializing Shader...</span>
      <a class="pill" href="https://github.com/witt3rd/lacuna" target="_blank" rel="noopener">GitHub: witt3rd/lacuna</a>
    </div>
    <div class="controls">
      <label>Whitespace Gain <input id="gain" type="range" min="0.4" max="2.5" step="0.05" value="1.35"/></label>
      <label>Supply Sigma <input id="sigmaS" type="range" min="0.08" max="0.35" step="0.01" value="0.16"/></label>
      <label><input id="showVor" type="checkbox" checked/> Voronoi &amp; Delaunay Tethers</label>
      <label><input id="showLabels" type="checkbox" checked/> Labels</label>
    </div>
  </header>
  <div id="canvas-wrap">
    <canvas id="gpu-canvas"></canvas>
    <canvas id="overlay"></canvas>
    <div id="hud-legend">
      <div><span style="color:#00f5d4">●</span> <strong>Cyan Star + Dashed Cell</strong>: Interior Delaunay Whitespace Void (click to inspect)</div>
      <div><span style="color:#f43f5e">●</span> <strong>Rose Node</strong>: Existing Steam Game Cluster (click to inspect facets)</div>
      <div><span style="color:#00f5d4">■</span> <strong>Cyan Basin</strong>: High Demand Gravity × Low Incumbent Supply</div>
    </div>
  </div>
</div>
<aside>
  <div class="card">
    <h2>RANKED INTERIOR VOIDS (CLICK TO EXPLORE)</h2>
    <div id="void-list" class="void-list"></div>
  </div>
  <div class="card" id="inspector"></div>
  <div class="card">
    <h2>WGSL TOPOGRAPHIC POTENTIAL FIELD SHADER</h2>
    <pre id="wgsl-box"></pre>
  </div>
</aside>
<script>
const BUNDLE = {data_json};
const WGSL_SOURCE = {wgsl_escaped};
document.getElementById('wgsl-box').textContent = WGSL_SOURCE.trim();

let selectedVoidIdx = 0;
let selectedEntityIdx = null;
let hoveredItem = null;

const gpuCanvas = document.getElementById('gpu-canvas');
const overlay = document.getElementById('overlay');
const ctx = overlay.getContext('2d');
const gainInput = document.getElementById('gain');
const sigmaSInput = document.getElementById('sigmaS');
const showVorInput = document.getElementById('showVor');
const showLabelsInput = document.getElementById('showLabels');
const gpuBadge = document.getElementById('gpu-badge');

let webgpuState = null;

async function initWebGPU() {{
  if (!navigator.gpu) return false;
  try {{
    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) return false;
    const device = await adapter.requestDevice();
    const context = gpuCanvas.getContext('webgpu');
    if (!context) return false;
    const format = navigator.gpu.getPreferredCanvasFormat();
    context.configure({{ device, format, alphaMode: 'opaque' }});

    const shaderModule = device.createShaderModule({{ code: WGSL_SOURCE }});
    const pipeline = device.createRenderPipeline({{
      layout: 'auto',
      vertex: {{ module: shaderModule, entryPoint: 'vs_main' }},
      fragment: {{ module: shaderModule, entryPoint: 'fs_main', targets: [{{ format }}] }},
      primitive: {{ topology: 'triangle-list' }},
    }});

    const entities = BUNDLE.snapshot.entities;
    const coords = BUNDLE.entity_coords_2d;
    const entityData = new Float32Array(entities.length * 4);
    for (let i = 0; i < entities.length; i++) {{
      entityData[i * 4 + 0] = coords[i][0];
      entityData[i * 4 + 1] = coords[i][1];
      entityData[i * 4 + 2] = entities[i].demand_signal;
      entityData[i * 4 + 3] = entities[i].quality_signal;
    }}

    const entityBuffer = device.createBuffer({{
      size: Math.max(entityData.byteLength, 16),
      usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
    }});
    device.queue.writeBuffer(entityBuffer, 0, entityData);

    const uniformBuffer = device.createBuffer({{
      size: 16,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    }});

    const bindGroup = device.createBindGroup({{
      layout: pipeline.getBindGroupLayout(0),
      entries: [
        {{ binding: 0, resource: {{ buffer: entityBuffer }} }},
        {{ binding: 1, resource: {{ buffer: uniformBuffer }} }},
      ],
    }});

    webgpuState = {{ device, context, pipeline, uniformBuffer, bindGroup, count: entities.length }};
    gpuBadge.textContent = 'WebGPU WGSL Active';
    return true;
  }} catch (err) {{
    console.warn('WebGPU fallback to Canvas2D:', err);
    return false;
  }}
}}

function toPx(pt) {{
  const w = overlay.width, h = overlay.height;
  return [(pt[0] + 1) * 0.5 * w, (1 - (pt[1] + 1) * 0.5) * h];
}}

function fromPx(px, py) {{
  const w = overlay.width, h = overlay.height;
  return [(px / w) * 2 - 1, 1 - (py / h) * 2];
}}

function renderWebGPUField(gain, sigmaS) {{
  if (!webgpuState) return false;
  const {{ device, context, pipeline, uniformBuffer, bindGroup, count }} = webgpuState;
  const buf = new ArrayBuffer(16);
  new Uint32Array(buf, 0, 1)[0] = count;
  new Float32Array(buf, 4, 3).set([sigmaS, 0.34, gain]);
  device.queue.writeBuffer(uniformBuffer, 0, buf);

  const encoder = device.createCommandEncoder();
  const pass = encoder.beginRenderPass({{
    colorAttachments: [{{
      view: context.getCurrentTexture().createView(),
      loadOp: 'clear',
      clearValue: {{ r: 0.03, g: 0.05, b: 0.08, a: 1.0 }},
      storeOp: 'store',
    }}],
  }});
  pass.setPipeline(pipeline);
  pass.setBindGroup(0, bindGroup);
  pass.draw(6);
  pass.end();
  device.queue.submit([encoder.finish()]);
  return true;
}}

function draw() {{
  const w = overlay.width, h = overlay.height;
  ctx.clearRect(0, 0, w, h);
  const gain = parseFloat(gainInput.value);
  const sigmaS = parseFloat(sigmaSInput.value);
  const sigmaD = 0.34;

  const entities = BUNDLE.snapshot.entities;
  const coords = BUNDLE.entity_coords_2d;

  if (!renderWebGPUField(gain, sigmaS)) {{
    gpuBadge.textContent = 'Shader Canvas2D Active';
    const inv2S = 1 / (2 * sigmaS * sigmaS);
    const inv2D = 1 / (2 * sigmaD * sigmaD);
    const grid = 64;
    const cw = Math.ceil(w / grid), ch = Math.ceil(h / grid);

    for (let r = 0; r < grid; r++) {{
      const wy = 1 - 2 * (r + 0.5) / grid;
      for (let c = 0; c < grid; c++) {{
        const wx = -1 + 2 * (c + 0.5) / grid;
        let sup = 0, dem = 0;
        for (let i = 0; i < entities.length; i++) {{
          const dx = wx - coords[i][0], dy = wy - coords[i][1];
          const r2 = dx * dx + dy * dy;
          sup += Math.exp(-r2 * inv2S);
          const pull = Math.max(entities[i].demand_signal, 0.1) * entities[i].quality_signal;
          const ring = Math.min(r2 / (sigmaD * sigmaD), 2.5) * Math.exp(-r2 * inv2D);
          dem += pull * ring;
        }}
        const ns = Math.min(sup / Math.max(entities.length * 0.18, 1), 1);
        const nd = Math.min(dem / Math.max(entities.length * 1.4, 1), 1);
        const env = Math.max(1 - 0.35 * (wx * wx + wy * wy), 0.1);
        const ws = Math.min(nd * Math.pow(1 - ns, 1.35) * env * gain, 1);

        const red = Math.round(10 + 195 * ns);
        const grn = Math.round(16 + 220 * ws);
        const blu = Math.round(28 + 225 * ws + 55 * ns);
        ctx.fillStyle = `rgb(${{red}},${{grn}},${{blu}})`;
        ctx.fillRect(c * (w / grid), r * (h / grid), cw + 1, ch + 1);
      }}
    }}
  }}

  const dpr = window.devicePixelRatio || 1;
  const activeVoid = BUNDLE.voids[selectedVoidIdx];
  if (showVorInput.checked && activeVoid && activeVoid.voronoi_polygon.length >= 3) {{
    ctx.beginPath();
    activeVoid.voronoi_polygon.forEach((pt, idx) => {{
      const [px, py] = toPx(pt);
      if (idx === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
    }});
    ctx.closePath();
    ctx.fillStyle = 'rgba(0, 245, 212, 0.14)';
    ctx.fill();
    ctx.strokeStyle = '#00f5d4';
    ctx.lineWidth = 2 * dpr;
    ctx.setLineDash([6 * dpr, 4 * dpr]);
    ctx.stroke();
    ctx.setLineDash([]);

    const [vx, vy] = toPx(activeVoid.coords_2d);
    activeVoid.boundary_neighbors.forEach(nb => {{
      const idx = entities.findIndex(e => e.id === nb.entity_id);
      if (idx >= 0) {{
        const [nx, ny] = toPx(coords[idx]);
        ctx.beginPath();
        ctx.moveTo(vx, vy);
        ctx.lineTo(nx, ny);
        ctx.strokeStyle = 'rgba(0, 245, 212, 0.78)';
        ctx.lineWidth = 1.6 * dpr;
        ctx.stroke();
      }}
    }});
  }}

  entities.forEach((e, i) => {{
    const [px, py] = toPx(coords[i]);
    const isSelected = selectedEntityIdx === i;
    const isSynthetic = e.metadata && e.metadata.synthetic_cluster_of;
    const radius = (isSelected ? 7.5 : (isSynthetic ? 3.5 : 5.5)) * dpr;
    ctx.beginPath();
    ctx.arc(px, py, radius, 0, Math.PI * 2);
    ctx.fillStyle = isSelected ? '#f59e0b' : (isSynthetic ? 'rgba(244, 63, 94, 0.65)' : '#f43f5e');
    ctx.fill();
    ctx.strokeStyle = isSelected ? '#fff' : 'rgba(255,255,255,0.75)';
    ctx.lineWidth = (isSelected ? 2 : 1) * dpr;
    ctx.stroke();

    if (showLabelsInput.checked && (!isSynthetic || isSelected)) {{
      ctx.fillStyle = isSelected ? '#fde68a' : '#e2e8f0';
      ctx.font = `${{11 * dpr}}px monospace`;
      ctx.fillText(e.name, px + (radius + 5 * dpr), py + 3.5 * dpr);
    }}
  }});

  BUNDLE.voids.forEach((v, idx) => {{
    const [vx, vy] = toPx(v.coords_2d);
    const active = idx === selectedVoidIdx && selectedEntityIdx === null;
    ctx.beginPath();
    ctx.arc(vx, vy, (active ? 9.5 : 6.5) * dpr, 0, Math.PI * 2);
    ctx.fillStyle = active ? '#00f5d4' : '#38bdf8';
    ctx.fill();
    ctx.strokeStyle = '#070b12';
    ctx.lineWidth = 2.2 * dpr;
    ctx.stroke();

    if (showLabelsInput.checked) {{
      ctx.fillStyle = active ? '#00f5d4' : '#7dd3fc';
      ctx.font = `bold ${{11 * dpr}}px monospace`;
      ctx.fillText(`#${{v.rank}} ${{v.void_id}}`, vx + 11 * dpr, vy - 5 * dpr);
    }}
  }});
}}

overlay.addEventListener('click', (ev) => {{
  const rect = overlay.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  const mx = (ev.clientX - rect.left) * dpr;
  const my = (ev.clientY - rect.top) * dpr;

  // 1. Check void centers first
  for (let i = 0; i < BUNDLE.voids.length; i++) {{
    const [vx, vy] = toPx(BUNDLE.voids[i].coords_2d);
    if (Math.hypot(mx - vx, my - vy) <= 14 * dpr) {{
      selectedVoidIdx = i;
      selectedEntityIdx = null;
      renderSidebar();
      draw();
      return;
    }}
  }}

  // 2. Check corpus entities
  for (let i = 0; i < BUNDLE.snapshot.entities.length; i++) {{
    const [ex, ey] = toPx(BUNDLE.entity_coords_2d[i]);
    if (Math.hypot(mx - ex, my - ey) <= 10 * dpr) {{
      selectedEntityIdx = i;
      renderSidebar();
      draw();
      return;
    }}
  }}
}});

function renderSidebar() {{
  const listEl = document.getElementById('void-list');
  listEl.innerHTML = '';
  BUNDLE.voids.forEach((v, idx) => {{
    const opp = BUNDLE.opportunities.find(o => o.void_id === v.void_id);
    const btn = document.createElement('button');
    btn.className = 'void-btn' + (idx === selectedVoidIdx && selectedEntityIdx === null ? ' active' : '');
    const title = opp ? opp.hypothesis.title : v.void_id;
    btn.innerHTML = `<strong>#${{v.rank}} ${{v.void_id}}</strong> — ${{title}}<br/><span style="color:#94a3b8">Radius: ${{v.radius.toFixed(3)}} | Opportunity: ${{v.opportunity_score.toFixed(3)}}</span>`;
    btn.onclick = () => {{ selectedVoidIdx = idx; selectedEntityIdx = null; renderSidebar(); draw(); }};
    listEl.appendChild(btn);
  }});

  const insp = document.getElementById('inspector');

  if (selectedEntityIdx !== null) {{
    const ent = BUNDLE.snapshot.entities[selectedEntityIdx];
    const tagsHtml = ent.tags.map(t => `<span class="badge">${{t}}</span>`).join('');
    const facetsHtml = Object.entries(ent.facets).map(([k, val]) =>
      `<div style="margin-bottom:6px"><strong style="color:#f43f5e">${{k}}</strong>: ${{val}}</div>`
    ).join('');
    insp.innerHTML = `
      <h2>EXISTING CORPUS ENTITY (${{ent.id}})</h2>
      <h3>${{ent.name}}</h3>
      <div style="margin-bottom:8px">${{tagsHtml}}</div>
      <p style="margin-bottom:10px;color:#e2e8f0">${{ent.summary}}</p>
      <h2>SIGNALS</h2>
      <p style="margin-bottom:10px">Demand Signal: <strong>${{ent.demand_signal.toFixed(2)}}</strong> | Quality Ratio: <strong>${{(ent.quality_signal * 100).toFixed(0)}}%</strong></p>
      <h2>DECOMPOSED DESIGN FACETS</h2>
      <p>${{facetsHtml}}</p>
    `;
    return;
  }}

  const v = BUNDLE.voids[selectedVoidIdx];
  const opp = BUNDLE.opportunities.find(o => o.void_id === v.void_id);
  if (!v) return;

  const neighborsHtml = v.boundary_neighbors.map(n =>
    `<div style="margin-bottom:6px">• <strong>${{n.name}}</strong> (d=${{n.distance.toFixed(3)}}, simplex weight=${{n.simplex_weight.toFixed(2)}})</div>`
  ).join('');

  if (opp) {{
    const d = opp.decision;
    const p = opp.closed_loop_proof;
    const tagsHtml = opp.hypothesis.tags.map(t => `<span class="badge">${{t}}</span>`).join('');
    insp.innerHTML = `
      <h2>SYNTHESIZED OPPORTUNITY (RUNG LADDER VERIFIED)</h2>
      <h3>${{opp.hypothesis.title}}</h3>
      <div style="margin-bottom:8px">${{tagsHtml}}</div>
      <p style="margin-bottom:10px;color:#e2e8f0">${{opp.hypothesis.high_concept_logline}}</p>
      <h2>CORE 30-SECOND LOOP</h2>
      <p style="margin-bottom:10px">${{opp.hypothesis.core_30s_loop}}</p>
      <h2>RUNG_STD::DECIDE VIABILITY DISTRIBUTIONS (${{d.judge_model}})</h2>
      <p>Mechanically Coherent: <strong>${{(d.p_mechanically_coherent * 100).toFixed(0)}}%</strong></p>
      <div class="meter"><div style="width:${{d.p_mechanically_coherent * 100}}%"></div></div>
      <p>Genuinely Novel: <strong>${{(d.p_genuinely_novel * 100).toFixed(0)}}%</strong></p>
      <div class="meter"><div style="width:${{d.p_genuinely_novel * 100}}%"></div></div>
      <p>Commercial Hunger: <strong>${{(d.p_commercial_hunger * 100).toFixed(0)}}%</strong></p>
      <div class="meter"><div style="width:${{d.p_commercial_hunger * 100}}%"></div></div>
      <p style="margin-bottom:10px">Scope Tier: <strong>${{d.scope_tier}}</strong> (Composite Viability: <strong>${{d.composite_viability.toFixed(3)}}</strong>)</p>
      <h2>CLOSED-LOOP RE-EMBEDDING PROOF</h2>
      <p style="margin-bottom:10px">Verified in Void: <strong style="color:#00f5d4">${{p.verified_in_void}}</strong> | Nearest Existing Entity: <strong>${{p.closest_existing_entity_name}}</strong> (d=${{p.min_distance_to_existing_entity.toFixed(3)}})</p>
      <h2>DELAUNAY BOUNDARY NEIGHBORS</h2>
      <p>${{neighborsHtml}}</p>
    `;
  }} else {{
    insp.innerHTML = `<h2>${{v.void_id}}</h2><p>${{neighborsHtml}}</p>`;
  }}
}}

function resize() {{
  const dpr = window.devicePixelRatio || 1;
  const w = Math.max(1, Math.floor(overlay.clientWidth * dpr));
  const h = Math.max(1, Math.floor(overlay.clientHeight * dpr));
  gpuCanvas.width = w;
  gpuCanvas.height = h;
  overlay.width = w;
  overlay.height = h;
  draw();
}}

window.addEventListener('resize', resize);
[gainInput, sigmaSInput, showVorInput, showLabelsInput].forEach(el => el.addEventListener('input', draw));

initWebGPU().then(() => {{
  renderSidebar();
  resize();
}});
</script>
</body>
</html>
"##
    );

    Ok(html)
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use lacuna_core::{
        VoidSolverConfig, compute_field_grid, find_interior_voids, project_entities_2d,
        steam_seed_snapshot,
    };
    use lacuna_rung::{CalibratedGeometricDecider, SynthesisSpec, run_synthesis_ladder};

    use super::*;

    #[test]
    fn renders_svg_and_webgpu_html_explorer_from_bundle() {
        let snapshot = steam_seed_snapshot();
        let weights = snapshot.schema.default_weights();
        let coords = project_entities_2d(&snapshot.entities, &weights);
        let field = compute_field_grid(&snapshot.entities, &coords, 12);
        let voids = find_interior_voids(
            &snapshot.schema,
            &snapshot.entities,
            &VoidSolverConfig {
                top_k: 3,
                ..Default::default()
            },
        );
        let opp = run_synthesis_ladder(SynthesisSpec {
            schema: snapshot.schema.clone(),
            corpus: Arc::new(snapshot.entities.clone()),
            void: voids[0].clone(),
            decider: Arc::new(CalibratedGeometricDecider),
            live_llm: false,
            min_corpus_separation: 0.10,
            min_viability: 0.40,
            max_attempts: 2,
        })
        .expect("synthesis succeeds");

        let bundle = ExplorerBundle {
            snapshot,
            entity_coords_2d: coords,
            field,
            voids,
            opportunities: vec![opp],
        };

        let svg = render_svg_manifold(&bundle, 800, 600);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Factorio"));
        assert!(svg.contains("void-01"));

        let html = render_webgpu_explorer_html(&bundle).expect("html renders");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("@fragment"));
        assert!(html.contains("CLOSED-LOOP RE-EMBEDDING PROOF"));
    }
}
