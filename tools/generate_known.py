#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Original procedural designs; regenerate STL only, leaving design-answer JSON untouched."""
import math
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[1] / "fixtures" / "known"
N = 64


class Part:
    def __init__(self):
        self.triangles = []

    def triangle(self, a, b, c):
        self.triangles.append((a, b, c))

    def quad(self, a, b, c, d):
        self.triangle(a, b, c)
        self.triangle(a, c, d)

    def write(self, name):
        header = f"SPDX-License-Identifier: MIT OR Apache-2.0; {name}".encode()
        with (ROOT / f"{name}.stl").open("wb") as out:
            out.write(header.ljust(80, b" "))
            out.write(struct.pack("<I", len(self.triangles)))
            for triangle in self.triangles:
                coords = [v for point in triangle for v in point]
                out.write(struct.pack("<12fH", 0, 0, 0, *coords, 0))
        print(f"{name}: {len(self.triangles)} triangles")


def ring(radius, z):
    return [(radius * math.cos(i * math.tau / N), radius * math.sin(i * math.tau / N), z)
            for i in range(N)]


def join(part, a, b, reverse=False):
    for i in range(len(a)):
        j = (i + 1) % len(a)
        if reverse:
            part.quad(a[i], b[i], b[j], a[j])
        else:
            part.quad(a[i], a[j], b[j], b[i])


def cap(part, points, z, top):
    for i in range(len(points)):
        j = (i + 1) % len(points)
        a, b = (points[i], points[j]) if top else (points[j], points[i])
        part.triangle((0, 0, z), a, b)


def stepped_cylinder():
    part = Part()
    profile = [(10, z) for z in [0, 2, 4, 6, 8, 10]]
    profile += [(6, z) for z in [10, 12, 14, 16, 18, 20]]
    rings = [ring(r, z) for r, z in profile]
    for a, b in zip(rings, rings[1:]):
        join(part, a, b)
    cap(part, rings[0], 0, False)
    cap(part, rings[-1], 20, True)
    return part


def sphere_seat():
    part = Part()
    outer_bottom, outer_top = ring(15, -3), ring(15, 6)
    join(part, outer_bottom, outer_top)
    cap(part, outer_bottom, -3, False)
    bowls = []
    for degrees in [170, 160, 150, 140, 130, 120]:
        angle = math.radians(degrees)
        bowls.append(ring(12 * math.sin(angle), 12 + 12 * math.cos(angle)))
    # Concave surface faces the centre (upward at the pole).
    cap(part, bowls[0], 0, True)
    for a, b in zip(bowls, bowls[1:]):
        join(part, a, b, reverse=True)
    join(part, outer_top, bowls[-1])
    return part


def bracket():
    part = Part()
    polygon = [(0, 0), (40, 0), (40, 20), (32, 20), (32, 8), (8, 8), (8, 20), (0, 20)]
    front = [(x, 0, z) for x, z in polygon]
    back = [(x, 20, z) for x, z in polygon]
    for i in range(len(polygon)):
        j = (i + 1) % len(polygon)
        part.quad(front[i], back[i], back[j], front[j])
    # Ear clipping of this simple, counter-clockwise eight-corner cross section.
    def turn(a, b, c):
        return (b[0]-a[0])*(c[1]-a[1]) - (b[1]-a[1])*(c[0]-a[0])
    ids = list(range(len(polygon)))
    while len(ids) > 3:
        for k, b in enumerate(ids):
            a, c = ids[k-1], ids[(k+1) % len(ids)]
            if turn(polygon[a], polygon[b], polygon[c]) <= 0:
                continue
            def inside(p):
                return all(turn(polygon[u], polygon[v], p) >= 0
                           for u, v in [(a, b), (b, c), (c, a)])
            if any(inside(polygon[p]) for p in ids if p not in (a, b, c)):
                continue
            part.triangle(front[a], front[b], front[c])
            part.triangle(back[c], back[b], back[a])
            ids.pop(k)
            break
        else:
            raise ValueError("bracket cross section has no valid ear")
    a, b, c = ids
    part.triangle(front[a], front[b], front[c])
    part.triangle(back[c], back[b], back[a])
    return part


def hole_block():
    part = Part()
    angles = {i * math.tau / N for i in range(N)}
    angles.update(math.atan2(y, x) % math.tau for x in [-10, 10] for y in [-15, 15])
    angles = sorted(angles)
    for center_x, radius in [(10, 1.5), (30, 2.5), (50, 4)]:
        outer, inner = [], []
        for a in angles:
            x, y = math.cos(a), math.sin(a)
            t = min(10 / abs(x) if x else math.inf, 15 / abs(y) if y else math.inf)
            # Exact shared rectangle boundaries, including every rectangle corner.
            outer.append((round(center_x + t*x, 9), round(15 + t*y, 9)))
            inner.append((center_x + radius*x, 15 + radius*y))
        low = [(x, y, 0) for x, y in inner]
        for z in [2, 4, 6, 8, 10]:
            high = [(x, y, z) for x, y in inner]
            join(part, low, high, reverse=True)
            low = high
        for i in range(len(angles)):
            j = (i+1) % len(angles)
            oi, oj, ii, ij = outer[i], outer[j], inner[i], inner[j]
            for z in [0, 10]:
                vertices = [(*oi, z), (*oj, z), (*ij, z), (*ii, z)]
                if z == 0:
                    vertices.reverse()
                part.quad(*vertices)
            vertical = oi[0] == oj[0] and oi[0] in (0, 60)
            horizontal = oi[1] == oj[1] and oi[1] in (0, 30)
            if vertical or horizontal:
                part.quad((*oi, 0), (*oj, 0), (*oj, 10), (*oi, 10))
    return part


def main():
    ROOT.mkdir(parents=True, exist_ok=True)
    for name, make in [("hole-block", hole_block), ("stepped-cylinder", stepped_cylinder),
                       ("sphere-seat", sphere_seat), ("parallel-bracket", bracket)]:
        make().write(name)


if __name__ == "__main__":
    main()
