#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Turn a CAD mesh into a scan-like mesh: an even triangle mesh from marching cubes over the part's signed distance,
with Gaussian noise on every vertex.

    python3 tools/dev/synth_scan.py PART.stl OUT.stl [--cell 0.25] [--noise 0.02] [--seed 1] [--turn DEG]

Inside or outside comes exactly from vertical rays through the part; the distance from the nearest of many surface
samples (to its plane inside, to the sample outside), so the zero level sits on the faces and sharp edges. Marching cubes softens sharp edges by up to about a cell,
as a scanner does. `--turn` rotates the part about a tilted axis first, so it is not aligned to the grid or the axes.
A test tool, not shipped (DECISIONS.md D-016): needs numpy, SciPy and scikit-image. Slow (a minute or more per part);
use the result with `rulercraft solidcraft OUT.stl` and compare with the part's known sizes or SolidCraft's volume.
"""
import argparse, struct
import numpy as np
from scipy.spatial import cKDTree
from skimage.measure import marching_cubes


def read_stl(path):
    b = open(path, "rb").read()
    n = struct.unpack("<I", b[80:84])[0]
    a = np.frombuffer(b[84:84 + 50 * n], dtype=np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("x", "<u2")]))
    return a["v"].astype(np.float64)


def write_stl(path, tris):
    n = np.cross(tris[:, 1] - tris[:, 0], tris[:, 2] - tris[:, 0])
    n /= np.maximum(np.linalg.norm(n, axis=1, keepdims=True), 1e-30)
    rec = np.zeros(len(tris), dtype=np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("x", "<u2")]))
    rec["n"], rec["v"] = n, tris
    with open(path, "wb") as f:
        f.write(b"synth_scan".ljust(80, b" ") + struct.pack("<I", len(tris)) + rec.tobytes())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("part"); ap.add_argument("out")
    ap.add_argument("--cell", type=float, default=0.25)
    ap.add_argument("--noise", type=float, default=0.02)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--turn", type=float, default=0.0)
    a = ap.parse_args()
    rng = np.random.default_rng(a.seed)
    tris = read_stl(a.part)
    e1, e2 = tris[:, 1] - tris[:, 0], tris[:, 2] - tris[:, 0]
    cr = np.cross(e1, e2)
    area = np.linalg.norm(cr, axis=1)
    keep = area > 1e-12
    tris, e1, e2, cr, area = tris[keep], e1[keep], e2[keep], cr[keep], area[keep]
    normals = cr / area[:, None]
    # Surface samples: about 16 per cell face, area-weighted.
    count = int(area.sum() / (a.cell / 4.0) ** 2)
    pick = rng.choice(len(tris), size=count, p=area / area.sum())
    r1, r2 = rng.random(count), rng.random(count)
    flip = r1 + r2 > 1
    r1[flip], r2[flip] = 1 - r1[flip], 1 - r2[flip]
    pts = tris[pick, 0] + r1[:, None] * e1[pick] + r2[:, None] * e2[pick]
    nrm = normals[pick]
    tree = cKDTree(pts)
    # The grid is offset by a fraction of a cell so no face of the part lies on grid planes.
    lo = tris.reshape(-1, 3).min(0) - 3 * a.cell + a.cell * np.array([0.37, 0.61, 0.29])
    hi = tris.reshape(-1, 3).max(0) + 3 * a.cell
    axes = [np.arange(lo[k], hi[k] + a.cell, a.cell) for k in range(3)]
    grid = np.stack(np.meshgrid(*axes, indexing="ij"), -1).reshape(-1, 3)
    # Inside or outside, exactly: count where a vertical ray through each grid column crosses the part.
    nx, ny, nz = (len(ax) for ax in axes)
    inside = np.zeros((nx, ny, nz), dtype=bool)
    for t in tris:
        x0, y0 = t[:, 0].min(), t[:, 1].min()
        x1, y1 = t[:, 0].max(), t[:, 1].max()
        i0, i1 = np.searchsorted(axes[0], [x0, x1])
        j0, j1 = np.searchsorted(axes[1], [y0, y1])
        if i1 <= i0 or j1 <= j0:
            continue
        X, Y = np.meshgrid(axes[0][i0:i1], axes[1][j0:j1], indexing="ij")
        (ax_, ay), (bx, by), (cx, cy) = t[0, :2], t[1, :2], t[2, :2]
        den = (by - cy) * (ax_ - cx) + (cx - bx) * (ay - cy)
        if abs(den) < 1e-15:
            continue  # a wall seen edge-on from above
        l1 = ((by - cy) * (X - cx) + (cx - bx) * (Y - cy)) / den
        l2 = ((cy - ay) * (X - cx) + (ax_ - cx) * (Y - cy)) / den
        l3 = 1 - l1 - l2
        hit = (l1 >= 0) & (l2 >= 0) & (l3 > 0)
        if not hit.any():
            continue
        z = l1 * t[0, 2] + l2 * t[1, 2] + l3 * t[2, 2]
        for (a_, b_), zz in zip(np.argwhere(hit), z[hit]):
            inside[i0 + a_, j0 + b_, : np.searchsorted(axes[2], zz)] ^= True
    # Distance: to the nearest sample's plane inside (exact near faces), to the sample itself outside (rounds
    # convex edges as a distance should).
    d1, i1 = tree.query(grid, k=1)
    plane = np.abs(np.einsum("nj,nj->n", grid - pts[i1], nrm[i1]))
    occ = inside.reshape(-1)
    sdf = np.where(occ, -plane, d1)
    vol = sdf.reshape(nx, ny, nz)
    v, f, _, _ = marching_cubes(vol, level=0.0, spacing=(a.cell,) * 3)
    v += lo
    v += rng.normal(0.0, a.noise, v.shape) if a.noise > 0 else 0.0
    if a.turn:
        ax = np.array([1.0, 2.0, 3.0]) / np.sqrt(14.0)
        t = np.radians(a.turn)
        K = np.array([[0, -ax[2], ax[1]], [ax[2], 0, -ax[0]], [-ax[1], ax[0], 0]])
        R = np.eye(3) + np.sin(t) * K + (1 - np.cos(t)) * K @ K
        v = v @ R.T + np.array([12.5, -40.0, 7.0])
    out = v[f]
    # Wind outwards: the enclosed volume must come out positive.
    vol6 = np.einsum("ij,ij->i", out[:, 0], np.cross(out[:, 1], out[:, 2])).sum()
    if vol6 < 0:
        out = out[:, ::-1]
    write_stl(a.out, out)
    print(f"{len(f)} triangles, cell {a.cell} mm, noise {a.noise} mm")


main()
