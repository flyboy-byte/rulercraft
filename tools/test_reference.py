# SPDX-License-Identifier: MIT OR Apache-2.0
"""Small fixed-answer checks of the optional reference helper."""
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest

import reference


class ReferenceTests(unittest.TestCase):
    def test_numbers_and_refusals(self):
        # Fixed numbers, never obtained by running the engine.
        checks = [{"pointer": "/fit/radius", "expected": 10, "tolerance": 0.001,
                   "reason": "reference rounded to three decimals"}]
        self.assertEqual(reference.compare({"fit": {"radius": 10.0005}}, checks), 1)
        for value in (10.002, float("nan"), None, True):
            with self.assertRaises(ValueError):
                reference.compare({"fit": {"radius": value}}, checks)
        with self.assertRaises(ValueError):
            reference.compare({}, checks)
        for tolerance in (-1, float("inf"), True):
            with self.assertRaises(ValueError):
                reference.compare({"fit": {"radius": 10}},
                                  [dict(checks[0], tolerance=tolerance)])
        with self.assertRaises(ValueError):
            reference.compare({}, [])

    @unittest.skipUnless(os.environ.get("RULERCRAFT_TEST_BINARY"), "CLI binary supplied by cargo test")
    def test_real_cli_fixed_plane(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            mesh = root / "plane.obj"
            mesh.write_text("v 0 0 2\nv 6 0 2\nv 0 6 2\nf 1 2 3\n")
            capture = {"version": 1, "mesh": "plane.obj",
                       "sha256": hashlib.sha256(mesh.read_bytes()).hexdigest(),
                       "operation": "plane", "args": ["--sigma", "0"],
                       "checks": [{"pointer": "/fit/center/2", "expected": 2,
                                   "tolerance": 1e-10, "reason": "exact plane z=2; f64 roundoff"},
                                  {"pointer": "/fit/sigma", "expected": 0,
                                   "tolerance": 1e-10, "reason": "exact coplanar points"}]}
            path = root / "plane.json"
            path.write_text(json.dumps(capture))
            self.assertEqual(reference.run(root, os.environ["RULERCRAFT_TEST_BINARY"]), 2)
            capture.update(operation="info", args=[], checks=[
                {"pointer": "/vertices", "expected": 3, "tolerance": 0, "reason": "exact count"},
                {"pointer": "/bounds/max/0", "expected": 6, "tolerance": 0, "reason": "integer coordinates"}])
            path.write_text(json.dumps(capture))
            self.assertEqual(reference.run(root, os.environ["RULERCRAFT_TEST_BINARY"]), 2)

    def test_missing_directory_skips_but_bad_capture_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.assertEqual(reference.run(root / "absent", "unused"), 0)
            mesh = root / "part.obj"
            mesh.write_bytes(b"original bytes")
            capture = {"version": 1, "mesh": "part.obj",
                       "sha256": hashlib.sha256(mesh.read_bytes()).hexdigest(),
                       "operation": "info", "args": [],
                       "checks": [{"pointer": "/vertices", "expected": 8,
                                   "tolerance": 0, "reason": "exact count"}]}
            path = root / "part.json"
            path.write_text(json.dumps(capture))
            self.assertEqual(reference.load(path)[1], mesh)
            mesh.write_bytes(b"changed bytes")
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                reference.load(path)


if __name__ == "__main__":
    unittest.main()
