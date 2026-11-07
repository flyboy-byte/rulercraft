# Handoff

Last updated 2026-11-07, Codex. Main was `5166f05` when PR #42 was opened. Check #42's current
GitHub state first; its verified segmentation CLI/MCP front end is the last item authorized in the
current work order. PRs #38 (`history.undo` verification), #39 (`math.range`) and #40 (`cad.solidcraft`)
are merged.

- This session verified #34's owed history wrong-value run in #38. Changing the undo cursor decrement to
  a no-op made the stdio check fail; #38 records `history.undo` as verified.
- `math.range` now scales before normalizing, and both angle helpers use unit directions. Known-answer
  cases span ordinary, large and small finite scales; the smallest subnormal normalizes to finite unit
  length. The deliberate denominator mutation failed its test. #39 merged.
- PR #42 exposes `rulercraft segment <mesh>` and MCP `segment` over the existing library functions.
  Default mode is cylinders for scans; planes and joined strips are explicit. Joined strips stay opt-in
  for tessellated CAD meshes because scans can yield thousands of small patches. Synthetic cube and tube
  checks cover all modes; MCP is driven over stdio. The private vase scan was not opened or committed.
- #42 checks: `cargo fmt --all --check`, workspace clippy with `-D warnings`, full workspace tests,
  `python3 tools/plan.py render`, and `python3 tools/plan.py check` passed. A deliberate wrong-default
  run changed CLI default to planes and the CLI test failed; the source was restored.
- R1 still needs Logan's published-dimension exit run and a decision on clipping (`docs/findings/2026-11-07-clipping-on-real-scans.md`). The requested work for this session ends at segmentation front ends. Do not start the next roadmap feature without a new instruction.

Work branch: `codex/segment-frontends`, PR #42. If #42 is still open, its checks/report are the next
review step. Once merged, fetch main and run `python3 tools/plan.py next`; stop at that listing unless
Logan authorizes more work.
