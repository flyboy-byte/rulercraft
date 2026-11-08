// SPDX-License-Identifier: MIT OR Apache-2.0
// Pure geometry for the viewer: no DOM, no WebGL. Matrices are column-major, as WebGL wants.
(function (root) {
  'use strict';
  const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
  const add = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
  const mulS = (a, s) => [a[0] * s, a[1] * s, a[2] * s];
  const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
  const len = (a) => Math.hypot(a[0], a[1], a[2]);
  const norm = (a) => { const l = len(a); return l > 0 ? [a[0] / l, a[1] / l, a[2] / l] : [0, 0, 0]; };

  function perspective(fovy, aspect, near, far) {
    const f = 1 / Math.tan(fovy / 2), nf = 1 / (near - far);
    return new Float32Array([f / aspect, 0, 0, 0, 0, f, 0, 0, 0, 0, (far + near) * nf, -1, 0, 0, 2 * far * near * nf, 0]);
  }
  function lookAt(eye, target, up) {
    const z = norm(sub(eye, target)), x = norm(cross(up, z)), y = cross(z, x);
    return new Float32Array([x[0], y[0], z[0], 0, x[1], y[1], z[1], 0, x[2], y[2], z[2], 0,
      -dot(x, eye), -dot(y, eye), -dot(z, eye), 1]);
  }
  function mul(a, b) { // a * b
    const o = new Float32Array(16);
    for (let c = 0; c < 4; c++) for (let r = 0; r < 4; r++) {
      let s = 0;
      for (let k = 0; k < 4; k++) s += a[k * 4 + r] * b[c * 4 + k];
      o[c * 4 + r] = s;
    }
    return o;
  }
  function invert(m) { // general 4x4 inverse (Gauss-Jordan), null when singular
    const a = Array.from(m), inv = [1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1];
    for (let c = 0; c < 4; c++) {
      let p = c;
      for (let r = c + 1; r < 4; r++) if (Math.abs(a[c * 4 + r]) > Math.abs(a[c * 4 + p])) p = r;
      if (Math.abs(a[c * 4 + p]) < 1e-12) return null;
      if (p !== c) for (let k = 0; k < 4; k++) {
        [a[k * 4 + c], a[k * 4 + p]] = [a[k * 4 + p], a[k * 4 + c]];
        [inv[k * 4 + c], inv[k * 4 + p]] = [inv[k * 4 + p], inv[k * 4 + c]];
      }
      const d = a[c * 4 + c];
      for (let k = 0; k < 4; k++) { a[k * 4 + c] /= d; inv[k * 4 + c] /= d; }
      for (let r = 0; r < 4; r++) if (r !== c) {
        const f = a[c * 4 + r];
        if (f !== 0) for (let k = 0; k < 4; k++) { a[k * 4 + r] -= f * a[k * 4 + c]; inv[k * 4 + r] -= f * inv[k * 4 + c]; }
      }
    }
    return new Float32Array(inv);
  }
  function transform(m, p) { // point, with perspective divide
    const x = m[0]*p[0]+m[4]*p[1]+m[8]*p[2]+m[12], y = m[1]*p[0]+m[5]*p[1]+m[9]*p[2]+m[13];
    const z = m[2]*p[0]+m[6]*p[1]+m[10]*p[2]+m[14], w = m[3]*p[0]+m[7]*p[1]+m[11]*p[2]+m[15];
    return [x / w, y / w, z / w];
  }
  // Ray through normalised device coordinates (nx, ny in -1..1).
  function ray(invViewProj, nx, ny) {
    const a = transform(invViewProj, [nx, ny, -1]), b = transform(invViewProj, [nx, ny, 1]);
    return { origin: a, dir: norm(sub(b, a)) };
  }
  // Moller-Trumbore. Distance along the ray, or -1 for a miss.
  function rayTriangle(o, d, a, b, c) {
    const e1 = sub(b, a), e2 = sub(c, a), p = cross(d, e2), det = dot(e1, p);
    if (Math.abs(det) < 1e-14) return -1;
    const inv = 1 / det, s = sub(o, a), u = dot(s, p) * inv;
    if (u < 0 || u > 1) return -1;
    const q = cross(s, e1), v = dot(d, q) * inv;
    if (v < 0 || u + v > 1) return -1;
    const t = dot(e2, q) * inv;
    return t > 0 ? t : -1;
  }
  // Nearest triangle hit by the ray: {t, tri, point} or null. pos: Float32Array xyz, idx: Uint32Array.
  function pick(pos, idx, o, d) {
    let best = Infinity, tri = -1;
    const v = (i) => [pos[i * 3], pos[i * 3 + 1], pos[i * 3 + 2]];
    for (let t = 0; t < idx.length / 3; t++) {
      const h = rayTriangle(o, d, v(idx[t * 3]), v(idx[t * 3 + 1]), v(idx[t * 3 + 2]));
      if (h >= 0 && h < best) { best = h; tri = t; }
    }
    return tri < 0 ? null : { t: best, tri, point: add(o, mulS(d, best)) };
  }
  // Ids of the vertices within `r` of `c`.
  function brush(pos, c, r) {
    const out = [], r2 = r * r;
    for (let i = 0; i < pos.length / 3; i++) {
      const dx = pos[i*3] - c[0], dy = pos[i*3+1] - c[1], dz = pos[i*3+2] - c[2];
      if (dx*dx + dy*dy + dz*dz <= r2) out.push(i);
    }
    return out;
  }
  // Two unit vectors perpendicular to `axis` and to each other.
  function basis(axis) {
    const a = norm(axis), seed = Math.abs(a[0]) < 0.9 ? [1, 0, 0] : [0, 1, 0];
    const u = norm(cross(a, seed));
    return [u, cross(a, u)];
  }
  // Line segments (flat xyz pairs) of a circle in the plane spanned by u, v.
  function circle(c, u, v, r, n) {
    const out = [];
    for (let i = 0; i < n; i++) {
      const a0 = 2 * Math.PI * i / n, a1 = 2 * Math.PI * (i + 1) / n;
      const p = (a) => add(c, add(mulS(u, r * Math.cos(a)), mulS(v, r * Math.sin(a))));
      out.push(...p(a0), ...p(a1));
    }
    return out;
  }
  const api = { sub, add, mulS, dot, cross, len, norm, perspective, lookAt, mul, invert, transform, ray, rayTriangle, pick, brush, basis, circle };
  if (typeof module !== 'undefined' && module.exports) module.exports = api; else root.Geom = api;
})(typeof globalThis !== 'undefined' ? globalThis : this);
