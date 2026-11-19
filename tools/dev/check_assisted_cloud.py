# SPDX-License-Identifier: MIT OR Apache-2.0
"""Probe CLI fits on synthetic, unlabelled plate clouds. Print one JSON record per trial."""
import argparse
import itertools
import json
import math
from pathlib import Path
import random
import subprocess
import tempfile


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def sub(a, b):
    return [x - y for x, y in zip(a, b)]


def capture(seed, angle, noise, visible):
    """A 60 x 30 x 8 plate with a diameter-8 hole at (15, 12), seen from above."""
    rng = random.Random(seed)
    sin, cos = math.sin(math.radians(angle)), math.cos(math.radians(angle))

    def place(p):
        x, y, z = p
        x, y = cos*x - sin*y, sin*x + cos*y
        return [(x+42)*0.43, (cos*y-sin*z-31)*0.43, (sin*y+cos*z+65)*0.43]

    points = []
    for _ in range(4000):
        x, y = rng.uniform(0, 60), rng.uniform(0, 30)
        if math.hypot(x-15, y-12) > 4:
            points.append((x, y, 8))
    for _ in range(700):
        x, y, z = rng.uniform(0, 60), rng.uniform(0, 30), rng.uniform(0, 8)
        points.extend([(x, 0, z), (x, 30, z), (0, y, z), (60, y, z)])
    for _ in range(360):
        azimuth = rng.uniform(0, math.radians(visible))
        z = rng.uniform(4, 8)
        if visible:
            points.append((15+4*math.cos(azimuth), 12+4*math.sin(azimuth), z))

    cloud = [place([x+rng.gauss(0, noise) for x in p]) for p in points]
    rng.shuffle(cloud)
    # Simulated user picks, approximated independently in the scanner frame.
    pick_rng = random.Random(seed + 100)
    def pick(point, sigma):
        return place([v+pick_rng.gauss(0, sigma) for v in point])

    markers = [pick((0, 0, 8), 0.15), pick((60, 0, 8), 0.15)]
    picks = [pick(p, 0.35) for p in ((30, 0, 4), (30, 30, 4), (15, 12, 4))]
    return cloud, markers, picks, place


def probe(binary, path, kind, options):
    proc = subprocess.run([binary, "fit", kind, str(path), *options],
                          capture_output=True, text=True, timeout=15)
    if proc.returncode == 2:
        return {"refusal": proc.stderr.strip()}
    proc.check_returncode()
    return json.loads(proc.stdout)["fit"]


def run_case(binary, path, seed, angle, noise, visible, length):
    cloud, markers, picks, place = capture(seed, angle, noise, visible)
    factor = length / math.dist(*markers)
    def calibrated(p):
        return [factor * x for x in p]

    path.write_text("".join(" ".join(f"{x:.7f}" for x in calibrated(p))+"\n" for p in cloud))
    def near(p, radius):
        return ["--near", *map(str, calibrated(p)), str(radius)]

    fits = [probe(binary, path, "plane", near(p, 3)) for p in picks[:2]]
    args = [*near(picks[2], 5.4), "--known-diameter", "8"]
    hole = probe(binary, path, "cylinder", args)
    result = dict(seed=seed, angle=angle, noise=noise, visible=visible, anchor=length,
                  scale_ratio=factor*0.43)
    if all("center" in f for f in fits):
        # Extra diagnostic: projected center separation along the first fitted normal.
        result["width"] = abs(dot(sub(fits[1]["center"], fits[0]["center"]), fits[0]["normal"]))
    else:
        result["plane_refusals"] = [f["refusal"] for f in fits if "refusal" in f]
    if "diameter" in hole:
        result["diameter"] = hole["diameter"]
        result["selected_after_clipping"] = hole["used"]
        # Score the returned axis in the true physical frame, including calibration error.
        # The scanner origin is arbitrary. Compare offsets from the first reference marker.
        ref = calibrated(markers[0])
        true_offset = [v / 0.43 for v in sub(place((15, 12, 4)), place((0, 0, 8)))]
        delta = sub(sub(hole["axis_point"], ref), true_offset)
        along = dot(delta, hole["axis"])
        result["axis_miss"] = math.sqrt(sum((v-along*a)**2 for v,a in zip(delta,hole["axis"])))
        normal = sub(place((0, 0, 1)), place((0, 0, 0)))
        result["tilt_deg"] = math.degrees(math.acos(min(1, abs(dot(hole["axis"], normal))/0.43)))
    else:
        result["bore_refusal"] = hole["refusal"]
    return result


def check_hidden_floor(host, directory):
    """Evaluate two different solids against precisely the same partial surface samples."""
    cloud, _, _, _ = capture(7, 0, 0, 360)
    observed = [[x/0.43-42, y/0.43+31, z/0.43-65] for x, y, z in cloud]
    witness = [(19, 12, 1)]
    for name, floor in [("through", -1), ("blind", 3)]:
        script = {"commands": [
            {"command": "solid.box", "params": {"length": "60", "width": "30", "height": "8",
                                                "corner": [0, 0, 0], "name": "Plate"}},
            {"command": "solid.cylinder", "params": {"diameter": "8", "height": str(9-floor),
                                                      "base": [15, 12, floor], "operation": "cut"}},
        ]}
        script_path = directory / "plate.json"
        script_path.write_text(json.dumps(script))
        model = directory / "plate.sc"
        subprocess.run([host, "run", str(script_path), "--save", str(model), "--quiet"], check=True)
        measured = json.loads(subprocess.check_output([host, "eval", str(model)], text=True))
        volume = measured["total"]["volume_mm3"]
        expected = 60*30*8 - math.pi*4**2*(8-max(0, floor))
        if abs(volume-expected) > 1e-6:
            raise ValueError(f"{name} volume {volume}, expected {expected}")
        for label, rows in [("visible", observed), ("hidden_wall", witness)]:
            xyz = directory / "samples.xyz"
            xyz.write_text("".join(" ".join(map(str, p))+"\n" for p in rows))
            body = measured["bodies"][0]["name"]
            script_path.write_text(json.dumps({"commands": [
                {"command": "pointcloud.insert", "params": {"path": str(xyz), "name": "samples"}},
                {"command": "pointcloud.deviation", "params": {"cloud": "samples", "body": body,
                                                              "tolerance": 0.05}},
            ]}))
            output = directory / "deviation.json"
            subprocess.run([host, "run", str(script_path), "--in", str(model), "--results",
                            "--json-out", str(output)], check=True, stdout=subprocess.DEVNULL)
            result = json.loads(output.read_text())[-1]
            expected_distance = 1 if name == "blind" and label == "hidden_wall" else 0
            if (result["measured"] != len(rows) or result["failed"] or
                    abs(result["max_abs_mm"]-expected_distance) > 1e-6):
                raise ValueError(f"{name}, {label}: {result}")
            print(json.dumps(dict(design=name, volume=volume, samples=label,
                                  measured=result["measured"], max_abs_mm=result["max_abs_mm"])))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", help="path to the rulercraft CLI")
    parser.add_argument("--solidcraft", help="also check unseen geometry with SolidCraft CLI")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as tmp:
        if args.solidcraft:
            check_hidden_floor(args.solidcraft, Path(tmp))
        path = Path(tmp) / "plate.xyz"
        for case in itertools.product((7, 19, 47), (0, 37), (0.15, 0.6), (360, 90, 0), (60, 61, 66)):
            print(json.dumps(run_case(args.binary, path, *case)))


if __name__ == "__main__":
    main()
