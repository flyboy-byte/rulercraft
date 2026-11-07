# Local reference numbers

Feature: `reference.local`. Optional independent numbers for the same mesh and recipe, kept in
`fixtures/private/reference/<mesh>.json` (gitignored). No other program's name, model or output is
committed. Capture instructions specific to the machine belong in gitignored `local/` (D-004).

Run `cargo build -p rulercraft-cli`, then `python3 tools/reference.py`. The CLI integration test runs
this helper too. If no `*.json` captures exist it prints `SKIP local references` and succeeds. An
existing capture with missing mesh, changed mesh bytes, invalid JSON, failed command, missing field or
out-of-tolerance result fails. It never generates or updates expected answers. A missing reference is
optional evidence; a broken reference is a failure.

Version 1 capture shape (illustrative numbers only):

```json
{
  "version": 1,
  "mesh": "../part.stl",
  "sha256": "64 lowercase hex digits from the exact mesh bytes",
  "operation": "cylinder",
  "args": ["--units", "mm", "--sigma", "0", "--box", "-11", "-11", "1", "11", "11", "9"],
  "checks": [
    {"pointer": "/fit/radius", "expected": 10.0, "tolerance": 0.001,
     "reason": "independent radius displayed to three decimal places"}
  ]
}
```

`operation` is `info`, `plane`, `sphere` or `cylinder`. The latter three run `fit <kind>`. `args` are
individual CLI strings appended after the mesh path, never shell text. Mesh paths resolve relative to
the capture. SHA-256 pins the reference to the exact bytes; a mismatch needs a fresh independent
capture, not refreshed expected answers. All lengths in checks are mm and angles degrees; `--units`
only describes source coordinates. Selection and clipping must match the independent operation.

Checks are a nonempty list of scalar numeric JSON pointers into CLI output. Array indices select
components; `~0` and `~1` escape `~` and `/`. Expected and actual values must be finite numbers, never
booleans. Tolerances are finite nonnegative absolute differences and each has a nonempty reason,
chosen before comparing results; never loosen one to pass. Use separate component checks for vectors
and beware sign-equivalent axes/normals. Each capture is at most 1 MiB and each CLI call has a 10 s
limit. `--directory` and `--binary` support isolated checks. Only direct `*.json` children are loaded.

This is a test helper, with no new production command or engine dependency. No captured independent
numbers are bundled; synthetic helper checks prove the comparison and refusal behavior, not agreement
with an independently measured real mesh.
