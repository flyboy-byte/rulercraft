// SPDX-License-Identifier: MIT OR Apache-2.0
// Run with: node crates/rulercraft-gui/tests/geom.test.js  (checked by tests/viewer_js.rs when node exists)
const G = require('../assets/geom.js');
const assert = require('assert');
const close = (a, b, e = 1e-4) => assert(Math.abs(a - b) < e, `${a} vs ${b}`);

// A ray down -z through a triangle in the z = 5 plane hits at distance 5 (and misses outside it).
const tri = [[0, 0, 5], [10, 0, 5], [0, 10, 5]];
close(G.rayTriangle([2, 2, 10], [0, 0, -1], ...tri), 5);
assert.strictEqual(G.rayTriangle([8, 8, 10], [0, 0, -1], ...tri), -1);
assert.strictEqual(G.rayTriangle([2, 2, 10], [0, 0, 1], ...tri), -1); // pointing away

// The camera: looking at the origin from +x, the ray through the screen centre hits a plate at x = 3.
const proj = G.perspective(Math.PI / 4, 2, 0.1, 100), view = G.lookAt([20, 0, 0], [0, 0, 0], [0, 0, 1]);
const vp = G.mul(proj, view), inv = G.invert(vp);
assert(inv);
const r = G.ray(inv, 0, 0);
close(r.dir[0], -1); close(r.dir[1], 0); close(r.dir[2], 0);
const pos = new Float32Array([3, -5, -5, 3, 5, -5, 3, 0, 5]), idx = new Uint32Array([0, 1, 2]);
const hit = G.pick(pos, idx, r.origin, r.dir);
assert(hit && hit.tri === 0); close(hit.point[0], 3);
// Looking down -x with z up, screen right is +y: a ray through the right half bends toward +y.
const right = G.ray(inv, 0.5, 0);
assert(right.dir[1] > 0, 'screen right should point to +y when looking down -x with z up');

// vp * inv(vp) is the identity.
const id = G.mul(vp, inv);
for (let i = 0; i < 4; i++) for (let j = 0; j < 4; j++) close(id[i * 4 + j], i === j ? 1 : 0, 1e-3);
assert.strictEqual(G.invert(new Float32Array(16)), null);

// Brush: vertices of a 3x3 grid (spacing 1) within 1.01 of the centre vertex are the centre plus its 4 neighbours.
const grid = []; for (let j = 0; j < 3; j++) for (let i = 0; i < 3; i++) grid.push(i, j, 0);
assert.deepStrictEqual(G.brush(new Float32Array(grid), [1, 1, 0], 1.01), [1, 3, 4, 5, 7]);
assert.deepStrictEqual(G.brush(new Float32Array(grid), [1, 1, 0], 0.5), [4]);

// basis is perpendicular and unit for awkward axes; circle closes on itself at the right radius.
for (const a of [[0, 0, 1], [1, 0, 0], [1, 1, 1], [0, 1e-9, 1]]) {
  const [u, v] = G.basis(a);
  close(G.len(u), 1); close(G.len(v), 1); close(G.dot(u, v), 0); close(G.dot(u, G.norm(a)), 0);
}
const c = G.circle([1, 2, 3], [1, 0, 0], [0, 1, 0], 4, 16);
assert.strictEqual(c.length, 16 * 6);
close(Math.hypot(c[0] - 1, c[1] - 2), 4); close(c[c.length - 3], c[0]); close(c[c.length - 2], c[1]);
console.log('geom ok');
