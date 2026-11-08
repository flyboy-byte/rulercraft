// SPDX-License-Identifier: MIT OR Apache-2.0
// The RulerCraft viewer: hand-written WebGL2, no libraries. All measurements come from the local server.
(() => {
'use strict';
const G = Geom, $ = (id) => document.getElementById(id);
const S = { info: null, pos: null, idx: null, tris: 0, sel: new Set(), fits: [], regions: null, triRegion: null,
  cam: { yaw: -0.8, pitch: 0.5, dist: 100, target: [0, 0, 0] }, diag: 100, dirty: true, msgTimer: 0 };
const PALETTE = ['#4aa3ff','#ff9f43','#5fd38d','#ff6b9d','#b388ff','#ffd166','#2ec4b6','#ef476f','#8ecae6','#c0ca33','#f4a261','#9ba4ff'];
const FIT_COLORS = ['#ffd24a','#4de3ff','#ff7ad9','#8dff6b','#ff9f43','#b388ff'];
const hex = (h) => [parseInt(h.slice(1,3),16)/255, parseInt(h.slice(3,5),16)/255, parseInt(h.slice(5,7),16)/255];

function msg(text, kind) {
  const m = $('msg'); m.textContent = text || ''; m.className = kind || '';
  clearTimeout(S.msgTimer);
  if (kind === 'ok') S.msgTimer = setTimeout(() => { m.textContent = ''; }, 5000);
}
const f3 = (x) => { if (Math.abs(x) < 5e-4) x = 0; return Math.abs(x) >= 1000 ? x.toFixed(1) : x.toFixed(3); };
const v3 = (a) => '(' + a.map(f3).join(', ') + ')';

async function api(path, body) {
  const opt = body === undefined ? {} : { method: 'POST', body: typeof body === 'string' || body instanceof ArrayBuffer ? body : JSON.stringify(body) };
  const r = await fetch(path, opt);
  if (!r.ok) {
    let t = ''; try { t = (await r.json()).error; } catch (_) { /* not JSON */ }
    throw new Error(t || ('request failed (' + r.status + ')'));
  }
  return r;
}

// ---- WebGL ----
const canvas = $('gl'), gl = canvas.getContext('webgl2', { antialias: true });
if (!gl) { msg('This browser has no WebGL2, so the 3D view cannot start.', 'err'); }
function program(vs, fs) {
  const p = gl.createProgram();
  for (const [t, src] of [[gl.VERTEX_SHADER, vs], [gl.FRAGMENT_SHADER, fs]]) {
    const s = gl.createShader(t); gl.shaderSource(s, src); gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s));
    gl.attachShader(p, s);
  }
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
  return p;
}
let meshProg, flatProg, meshVao, meshCount = 0, posBuf, colBuf, selBuf, selCount = 0;
if (gl) {
  meshProg = program(`#version 300 es
in vec3 p; in vec3 c; uniform mat4 mv, proj; out vec3 vp; out vec3 vc;
void main(){ vec4 v = mv*vec4(p,1.0); vp = v.xyz; vc = c; gl_Position = proj*v; }`,
`#version 300 es
precision highp float; in vec3 vp; in vec3 vc; out vec4 o;
void main(){ vec3 n = normalize(cross(dFdx(vp), dFdy(vp))); float l = abs(dot(n, normalize(-vp)));
  o = vec4(vc*(0.30+0.70*l), 1.0); }`);
  flatProg = program(`#version 300 es
in vec3 p; uniform mat4 mvp; uniform float size; void main(){ gl_Position = mvp*vec4(p,1.0); gl_PointSize = size; }`,
`#version 300 es
precision mediump float; uniform vec3 col; out vec4 o; void main(){ o = vec4(col,1.0); }`);
  posBuf = gl.createBuffer(); colBuf = gl.createBuffer(); selBuf = gl.createBuffer();
}

function matrices() {
  const c = S.cam, cp = Math.cos(c.pitch);
  const eye = [c.target[0] + c.dist * cp * Math.cos(c.yaw), c.target[1] + c.dist * cp * Math.sin(c.yaw), c.target[2] + c.dist * Math.sin(c.pitch)];
  const aspect = canvas.width / Math.max(1, canvas.height);
  const proj = G.perspective(Math.PI / 4, aspect, Math.max(c.dist * 0.01, S.diag * 1e-4), c.dist + S.diag * 4);
  const mv = G.lookAt(eye, c.target, [0, 0, 1]);
  return { proj, mv, vp: G.mul(proj, mv), eye };
}

function uploadMesh() {
  const nt = S.tris, p = new Float32Array(nt * 9);
  for (let t = 0; t < nt; t++) for (let k = 0; k < 3; k++) {
    const v = S.idx[t * 3 + k];
    p[t * 9 + k * 3] = S.pos[v * 3]; p[t * 9 + k * 3 + 1] = S.pos[v * 3 + 1]; p[t * 9 + k * 3 + 2] = S.pos[v * 3 + 2];
  }
  gl.bindBuffer(gl.ARRAY_BUFFER, posBuf); gl.bufferData(gl.ARRAY_BUFFER, p, gl.STATIC_DRAW);
  meshCount = nt * 3; uploadColors();
}
function uploadColors() {
  const nt = S.tris, c = new Float32Array(nt * 9), base = [0.62, 0.68, 0.76];
  const pal = PALETTE.map(hex);
  for (let t = 0; t < nt; t++) {
    const r = S.triRegion ? S.triRegion[t] : -1, col = r >= 0 ? pal[r % pal.length] : base;
    for (let k = 0; k < 3; k++) c.set(col, t * 9 + k * 3);
  }
  gl.bindBuffer(gl.ARRAY_BUFFER, colBuf); gl.bufferData(gl.ARRAY_BUFFER, c, gl.STATIC_DRAW);
  S.dirty = true;
}
function uploadSelection() {
  const ids = Array.from(S.sel), a = new Float32Array(ids.length * 3);
  ids.forEach((v, i) => { a[i*3] = S.pos[v*3]; a[i*3+1] = S.pos[v*3+1]; a[i*3+2] = S.pos[v*3+2]; });
  gl.bindBuffer(gl.ARRAY_BUFFER, selBuf); gl.bufferData(gl.ARRAY_BUFFER, a, gl.DYNAMIC_DRAW);
  selCount = ids.length; $('selcount').textContent = selCount + ' vertices marked'; S.dirty = true;
  document.querySelectorAll('[data-fit]').forEach((b) => { b.disabled = selCount < 3; });
}

function draw() {
  S.dirty = false;
  const w = Math.max(1, Math.floor(canvas.clientWidth * devicePixelRatio)), h = Math.max(1, Math.floor(canvas.clientHeight * devicePixelRatio));
  if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; }
  gl.viewport(0, 0, w, h);
  const dark = !matchMedia('(prefers-color-scheme: light)').matches;
  dark ? gl.clearColor(0.08, 0.09, 0.11, 1) : gl.clearColor(0.93, 0.94, 0.96, 1);
  gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
  if (!S.pos) return;
  const m = matrices();
  gl.enable(gl.DEPTH_TEST);
  gl.useProgram(meshProg);
  gl.uniformMatrix4fv(gl.getUniformLocation(meshProg, 'mv'), false, m.mv);
  gl.uniformMatrix4fv(gl.getUniformLocation(meshProg, 'proj'), false, m.proj);
  for (const [name, buf] of [['p', posBuf], ['c', colBuf]]) {
    const loc = gl.getAttribLocation(meshProg, name);
    gl.bindBuffer(gl.ARRAY_BUFFER, buf); gl.enableVertexAttribArray(loc); gl.vertexAttribPointer(loc, 3, gl.FLOAT, false, 0, 0);
  }
  gl.drawArrays(gl.TRIANGLES, 0, meshCount);
  gl.useProgram(flatProg);
  gl.uniformMatrix4fv(gl.getUniformLocation(flatProg, 'mvp'), false, m.vp);
  const ploc = gl.getAttribLocation(flatProg, 'p'), colLoc = gl.getUniformLocation(flatProg, 'col'), sizeLoc = gl.getUniformLocation(flatProg, 'size');
  gl.disable(gl.DEPTH_TEST);
  for (const f of S.fits) {
    if (!f.lineBuf) continue;
    gl.bindBuffer(gl.ARRAY_BUFFER, f.lineBuf); gl.enableVertexAttribArray(ploc); gl.vertexAttribPointer(ploc, 3, gl.FLOAT, false, 0, 0);
    gl.uniform3fv(colLoc, hex(f.color)); gl.uniform1f(sizeLoc, 1);
    gl.drawArrays(gl.LINES, 0, f.lineCount);
  }
  if (selCount) {
    gl.bindBuffer(gl.ARRAY_BUFFER, selBuf); gl.enableVertexAttribArray(ploc); gl.vertexAttribPointer(ploc, 3, gl.FLOAT, false, 0, 0);
    gl.uniform3fv(colLoc, [1.0, 0.45, 0.1]); gl.uniform1f(sizeLoc, Math.max(3, 4 * devicePixelRatio));
    gl.drawArrays(gl.POINTS, 0, selCount);
  }
}
function loop() { if (S.dirty) draw(); requestAnimationFrame(loop); }

// ---- Loading a mesh ----
function resetView() {
  const b = S.info.bounds;
  const lo = b.min, hi = b.max;
  S.cam.target = [0, 1, 2].map((i) => (lo[i] + hi[i]) / 2);
  S.diag = Math.max(1e-6, Math.hypot(hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]));
  S.cam.dist = S.diag * 1.4; S.cam.yaw = -0.8; S.cam.pitch = 0.5; S.dirty = true;
  const r = $('radius'); $('radiusv').textContent = f3(S.diag * r.value / 1000) + ' mm';
}
async function loadCurrent() {
  const info = await (await api('/api/info')).json();
  if (!info.name) { $('empty').style.display = 'flex'; return; }
  const [pb, ib] = await Promise.all([api('/api/positions').then((r) => r.arrayBuffer()), api('/api/indices').then((r) => r.arrayBuffer())]);
  S.info = info; S.pos = new Float32Array(pb); S.idx = new Uint32Array(ib); S.tris = S.idx.length / 3;
  S.sel.clear(); S.fits.forEach(freeFit); S.fits = []; S.regions = null; S.triRegion = null;
  uploadMesh(); uploadSelection(); renderFits(); renderRegions(); resetView();
  const b = info.bounds, size = [0, 1, 2].map((i) => b.max[i] - b.min[i]);
  $('fileinfo').textContent = info.name + ' · ' + info.vertices.toLocaleString() + ' vertices · ' + info.triangles.toLocaleString() +
    ' triangles · ' + size.map((x) => f3(x)).join(' × ') + ' mm';
  $('empty').style.display = 'none';
  $('result').textContent = 'Mark a patch, then press Plane, Sphere or Cylinder.';
}
async function openFile(file) {
  try {
    msg('Reading ' + file.name + '…');
    const buf = await file.arrayBuffer();
    await api('/api/open?name=' + encodeURIComponent(file.name) + '&unit=' + $('unit').value, buf);
    await loadCurrent(); msg('Opened ' + file.name, 'ok');
  } catch (e) { msg(e.message, 'err'); }
}
$('openbtn').onclick = () => $('file').click();
$('file').onchange = (e) => { if (e.target.files[0]) openFile(e.target.files[0]); e.target.value = ''; };
addEventListener('dragover', (e) => { e.preventDefault(); $('drop').style.display = 'flex'; });
addEventListener('dragleave', (e) => { if (e.relatedTarget === null) $('drop').style.display = 'none'; });
addEventListener('drop', (e) => { e.preventDefault(); $('drop').style.display = 'none'; if (e.dataTransfer.files[0]) openFile(e.dataTransfer.files[0]); });

// ---- Camera and picking ----
let drag = null;
canvas.addEventListener('contextmenu', (e) => e.preventDefault());
canvas.addEventListener('pointerdown', (e) => { canvas.setPointerCapture(e.pointerId); drag = { x: e.clientX, y: e.clientY, sx: e.clientX, sy: e.clientY, button: e.button, moved: false, shift: e.shiftKey || e.altKey || e.ctrlKey }; });
canvas.addEventListener('pointermove', (e) => {
  if (!drag) return;
  const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
  if (Math.hypot(e.clientX - drag.sx, e.clientY - drag.sy) > 4) drag.moved = true;
  if (!drag.moved) return;
  drag.x = e.clientX; drag.y = e.clientY;
  const c = S.cam;
  if (drag.button === 0) { c.yaw -= dx * 0.008; c.pitch = Math.max(-1.55, Math.min(1.55, c.pitch + dy * 0.008)); }
  else { // pan in the screen plane
    const m = matrices(), right = [m.mv[0], m.mv[4], m.mv[8]], up = [m.mv[1], m.mv[5], m.mv[9]];
    const k = c.dist * 2 * Math.tan(Math.PI / 8) / Math.max(1, canvas.clientHeight); // world units per screen pixel at the target
    c.target = G.add(c.target, G.add(G.mulS(right, -dx * k), G.mulS(up, dy * k)));
  }
  S.dirty = true;
});
canvas.addEventListener('pointerup', (e) => {
  const d = drag; drag = null;
  if (d && !d.moved && d.button === 0 && S.pos) clickAt(e, d.shift);
});
canvas.addEventListener('wheel', (e) => { e.preventDefault(); S.cam.dist = Math.max(S.diag * 0.02, Math.min(S.diag * 20, S.cam.dist * Math.exp(e.deltaY * 0.0012))); S.dirty = true; }, { passive: false });

function clickAt(e, remove) {
  const rc = canvas.getBoundingClientRect();
  const nx = ((e.clientX - rc.left) / rc.width) * 2 - 1, ny = 1 - ((e.clientY - rc.top) / rc.height) * 2;
  const inv = G.invert(matrices().vp); if (!inv) return;
  const r = G.ray(inv, nx, ny), hit = G.pick(S.pos, S.idx, r.origin, r.dir);
  if (!hit) { msg('That click missed the part.'); return; }
  let ids;
  if (S.triRegion && $('regionpick').checked && S.triRegion[hit.tri] >= 0) {
    const reg = S.triRegion[hit.tri], set = new Set();
    for (let t = 0; t < S.tris; t++) if (S.triRegion[t] === reg) for (let k = 0; k < 3; k++) set.add(S.idx[t * 3 + k]);
    ids = Array.from(set); showRegion(reg);
  } else ids = G.brush(S.pos, hit.point, S.diag * $('radius').value / 1000);
  for (const v of ids) remove ? S.sel.delete(v) : S.sel.add(v);
  uploadSelection();
}
$('radius').oninput = () => { $('radiusv').textContent = f3(S.diag * $('radius').value / 1000) + ' mm'; };
$('selall').onclick = () => { if (!S.pos) return; for (let i = 0; i < S.pos.length / 3; i++) S.sel.add(i); uploadSelection(); };
$('selclear').onclick = () => { S.sel.clear(); uploadSelection(); };
$('fitview').onclick = () => { if (S.info) resetView(); };
addEventListener('keydown', (e) => { if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT') return; if (e.key === 'f') $('fitview').click(); if (e.key === 'Escape') $('selclear').click(); });
addEventListener('resize', () => { S.dirty = true; });

// ---- Fits, overlays and measurements ----
function describe(f) {
  if (f.kind === 'plane') return 'Plane  ' + f3(f.span_u) + ' × ' + f3(f.span_v) + ' mm patch\nnormal ' + v3(f.normal) + '\nflatness (max − min) ' + f3(f.form_error) + ' mm · σ ' + f3(f.sigma) + ' mm';
  if (f.kind === 'sphere') return 'Sphere  diameter ' + f3(f.diameter) + ' mm (radius ' + f3(f.radius) + ')\ncentre ' + v3(f.center) + '\nσ ' + f3(f.sigma) + ' mm · covers ' + f3(f.cap_angle_deg) + '° of cap';
  return 'Cylinder  diameter ' + f3(f.diameter) + ' mm (radius ' + f3(f.radius) + ')\nlength ' + f3(f.length) + ' mm · axis ' + v3(f.axis) + '\naxis passes ' + v3(f.axis_point) + '\ncovers ' + f3(f.coverage_deg) + '° around · σ ' + f3(f.sigma) + ' mm';
}
function overlayLines(f) {
  const L = [];
  if (f.kind === 'plane') {
    const c = f.center, hu = G.mulS(f.basis_u, f.span_u / 2), hv = G.mulS(f.basis_v, f.span_v / 2);
    const q = [G.add(G.add(c, hu), hv), G.sub(G.add(c, hv), hu), G.sub(G.sub(c, hu), hv), G.sub(G.add(c, hu), hv)];
    for (let i = 0; i < 4; i++) L.push(...q[i], ...q[(i + 1) % 4]);
    L.push(...c, ...G.add(c, G.mulS(f.normal, Math.max(f.span_u, f.span_v) * 0.25)));
  } else if (f.kind === 'sphere') {
    const c = f.center, r = f.radius;
    L.push(...G.circle(c, [1, 0, 0], [0, 1, 0], r, 48), ...G.circle(c, [1, 0, 0], [0, 0, 1], r, 48), ...G.circle(c, [0, 1, 0], [0, 0, 1], r, 48));
  } else {
    const [u, v] = G.basis(f.axis), a = G.norm(f.axis), h = f.length / 2, p0 = G.sub(f.axis_point, G.mulS(a, h)), p1 = G.add(f.axis_point, G.mulS(a, h));
    L.push(...G.circle(p0, u, v, f.radius, 48), ...G.circle(p1, u, v, f.radius, 48), ...p0, ...p1);
    for (const [cu, cv] of [[1, 0], [0, 1], [-1, 0], [0, -1]]) {
      const o = G.add(G.mulS(u, cu * f.radius), G.mulS(v, cv * f.radius)); L.push(...G.add(p0, o), ...G.add(p1, o));
    }
  }
  return new Float32Array(L);
}
function freeFit(f) { if (f.lineBuf) gl.deleteBuffer(f.lineBuf); }
function renderFits() {
  const box = $('fits'); box.textContent = '';
  S.fits.forEach((f, i) => {
    const d = document.createElement('div'); d.className = 'item' + (f.ticked ? ' picked' : '');
    const head = document.createElement('div');
    const cb = document.createElement('input'); cb.type = 'checkbox'; cb.checked = !!f.ticked; cb.onchange = () => { f.ticked = cb.checked; renderFits(); };
    const chip = document.createElement('span'); chip.className = 'chip'; chip.style.background = f.color;
    const b = document.createElement('b'); b.textContent = f.label + ' · ' + f.fit.kind; b.style.cursor = 'pointer'; b.onclick = () => { $('result').textContent = describe(f.fit); };
    const del = document.createElement('button'); del.textContent = '✕'; del.title = 'Remove'; del.style.float = 'right'; del.onclick = () => { freeFit(f); S.fits.splice(i, 1); renderFits(); S.dirty = true; };
    head.append(cb, ' ', chip, b, del);
    const sub = document.createElement('span'); sub.className = 'sub';
    sub.textContent = f.fit.kind === 'plane' ? f3(f.fit.span_u) + ' × ' + f3(f.fit.span_v) + ' mm' : '⌀ ' + f3(f.fit.diameter) + ' mm';
    d.append(head, sub); box.append(d);
  });
  $('measure').disabled = S.fits.filter((f) => f.ticked).length !== 2;
}
document.querySelectorAll('[data-fit]').forEach((b) => { b.disabled = true; b.onclick = async () => {
  const req = { kind: b.dataset.fit, vertices: Array.from(S.sel), sigma: Number($('sigma').value) };
  const kd = $('known').value; if (kd !== '') { if (!(Number(kd) > 0)) { msg('Designed diameter must be above 0.', 'err'); return; } if (req.kind !== 'plane') req.known_diameter = Number(kd); }
  try {
    const out = await (await api('/api/fit', req)).json();
    const label = String.fromCharCode(65 + (S.fits.length % 26)) + (S.fits.length >= 26 ? Math.floor(S.fits.length / 26) + 1 : '');
    const f = { label, req, fit: out.fit, color: FIT_COLORS[S.fits.length % FIT_COLORS.length], ticked: false };
    const lines = overlayLines(out.fit); f.lineBuf = gl.createBuffer(); f.lineCount = lines.length / 3;
    gl.bindBuffer(gl.ARRAY_BUFFER, f.lineBuf); gl.bufferData(gl.ARRAY_BUFFER, lines, gl.STATIC_DRAW);
    S.fits.push(f); renderFits(); $('result').textContent = label + ': ' + describe(out.fit) + '\n(' + out.selected + ' points)'; S.dirty = true; msg('');
  } catch (e) { msg(e.message, 'err'); }
}; });
$('measure').onclick = async () => {
  const two = S.fits.filter((f) => f.ticked);
  try {
    const out = await (await api('/api/measure', { first: two[0].req, second: two[1].req })).json(), r = out.relation;
    const lines = [two[0].label + ' ↔ ' + two[1].label];
    if (r.distance !== null) lines.push('distance ' + f3(r.distance) + ' mm');
    if (r.components) lines.push('x, y, z  ' + v3(r.components) + ' mm');
    if (r.angle_deg !== null) lines.push('angle ' + f3(r.angle_deg) + '°');
    lines.push('', r.convention); $('result').textContent = lines.join('\n'); msg('');
  } catch (e) { msg(e.message, 'err'); }
};

// ---- Regions ----
function showRegion(i) {
  const r = S.regions && S.regions[i]; if (!r) return;
  $('result').textContent = r.kind === 'plane' ? 'Flat region ' + (i + 1) + '\narea ' + f3(r.area) + ' mm²\nnormal ' + v3(r.normal) + '\nabout ' + f3(r.span_u) + ' × ' + f3(r.span_v) + ' mm'
    : 'Round region ' + (i + 1) + '\ndiameter ' + f3(r.diameter) + ' mm\narea ' + f3(r.area) + ' mm²\naxis ' + v3(r.axis) + '\ncovers ' + f3(r.coverage_deg) + '°';
}
function renderRegions() {
  const box = $('regions'); box.textContent = '';
  (S.regions || []).slice(0, 60).forEach((r, i) => {
    const d = document.createElement('div'); d.className = 'item'; d.style.cursor = 'pointer';
    const chip = document.createElement('span'); chip.className = 'chip'; chip.style.background = PALETTE[i % PALETTE.length];
    const b = document.createElement('b'); b.textContent = (r.kind === 'plane' ? 'Flat ' : 'Round ') + (i + 1);
    const sub = document.createElement('span'); sub.className = 'sub'; sub.textContent = r.kind === 'plane' ? f3(r.area) + ' mm²' : '⌀ ' + f3(r.diameter) + ' mm · ' + f3(r.area) + ' mm²';
    d.append(chip, b, sub); d.onclick = () => showRegion(i); box.append(d);
  });
  if (S.regions && S.regions.length > 60) { const p = document.createElement('div'); p.className = 'hint'; p.textContent = '…and ' + (S.regions.length - 60) + ' smaller regions'; box.append(p); }
  if (S.regions && !S.regions.length) box.textContent = 'No regions found.';
}
$('seg').onclick = async () => {
  if (!S.pos) return;
  try {
    msg('Finding regions…');
    const out = await (await api('/api/segment', { kind: $('segkind').value })).json();
    S.regions = out.regions; S.triRegion = Int32Array.from(out.triangle_region); uploadColors(); renderRegions(); msg(out.regions.length + ' regions found. Click one to mark it.', 'ok');
  } catch (e) { msg(e.message, 'err'); }
};
$('segclear').onclick = () => { S.regions = null; S.triRegion = null; if (S.pos) uploadColors(); renderRegions(); };

if (gl) { requestAnimationFrame(loop); loadCurrent().catch((e) => msg(e.message, 'err')); }
})();
