# SPDX-License-Identifier: MIT OR Apache-2.0
"""Compare CLI JSON against private, independently captured numeric answers."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent


def number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def compare(result, checks):
    """Compare scalar JSON pointers with absolute tolerances; never update answers."""
    if not isinstance(checks, list) or not checks:
        raise ValueError("checks must be a nonempty list")
    for check in checks:
        if not isinstance(check, dict):
            raise ValueError("each check must be an object")
        pointer = check.get("pointer")
        expected, tolerance = check.get("expected"), check.get("tolerance")
        if (not number(expected) or not number(tolerance) or tolerance < 0
                or not isinstance(check.get("reason"), str) or not check["reason"].strip()):
            raise ValueError("each check needs finite expected/tolerance, tolerance >= 0 and a reason")
        if not isinstance(pointer, str) or not pointer.startswith("/"):
            raise ValueError("pointer must start with /")
        actual = result
        try:
            for token in pointer[1:].split("/"):
                token = token.replace("~1", "/").replace("~0", "~")
                if isinstance(actual, list):
                    if not token.isascii() or not token.isdecimal():
                        raise ValueError("array pointer needs a nonnegative index")
                    actual = actual[int(token)]
                else:
                    actual = actual[token]
        except (KeyError, IndexError, TypeError) as error:
            raise ValueError(f"missing numeric field {pointer}") from error
        if not number(actual) or abs(actual - expected) > tolerance:
            raise ValueError(f"{pointer}: got {actual}, expected {expected} +/- {tolerance}")
    return len(checks)


def load(path):
    """Load a capture, pin it to exact mesh bytes and validate its CLI recipe."""
    if path.stat().st_size > 1024 * 1024:
        raise ValueError("reference JSON exceeds 1 MiB")
    capture = json.loads(path.read_text())
    if not isinstance(capture, dict) or capture.get("version") != 1:
        raise ValueError("reference version must be 1")
    mesh_name = capture.get("mesh")
    if not isinstance(mesh_name, str) or not mesh_name:
        raise ValueError("mesh must be a path relative to the capture")
    mesh = (path.parent / mesh_name).resolve()
    with mesh.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    if digest != capture.get("sha256"):
        raise ValueError(f"SHA-256 mismatch for {mesh}; recapture independently, do not refresh answers")
    operation = capture.get("operation")
    if operation not in ("info", "plane", "sphere", "cylinder"):
        raise ValueError("operation must be info, plane, sphere or cylinder")
    args = capture.get("args", [])
    if not isinstance(args, list) or not all(isinstance(v, str) for v in args):
        raise ValueError("args must be a list of CLI argument strings")
    return capture, mesh


def run(directory, binary):
    """Run every *.json capture, or visibly skip when no captures exist."""
    paths = sorted(directory.glob("*.json"))
    if not paths:
        print(f"SKIP local references: no *.json in {directory}")
        return 0
    total = 0
    for path in paths:
        try:
            capture, mesh = load(path)
            operation = capture["operation"]
            command = [binary, "info"] if operation == "info" else [binary, "fit", operation]
            command += [str(mesh), *capture.get("args", [])]
            output = subprocess.run(command, capture_output=True, text=True, timeout=10, check=True)
            count = compare(json.loads(output.stdout), capture.get("checks"))
        except (ValueError, OSError, subprocess.SubprocessError) as error:
            raise ValueError(f"{path}: {error}") from error
        total += count
        print(f"PASS {path.name}: {count} numeric comparisons")
    return total


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, default=ROOT / "fixtures/private/reference")
    parser.add_argument("--binary", default=str(ROOT / "target/debug/rulercraft"))
    args = parser.parse_args()
    try:
        run(args.directory, args.binary)
    except ValueError as error:
        parser.exit(1, f"reference: {error}\n")


if __name__ == "__main__":
    main()
