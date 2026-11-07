# Handoff

Last updated 2026-11-07, Codex.

- MCP #18 is ready and mergeable; hosted CI passed in 57 s. It exposes mesh.open/info, select,
  all three fits, two-selection measurement and guide over stdio. Tests cover all tools and schemas,
  bounded selection pages and failed-open state preservation. rmcp Apache-2.0; Tokio MIT.
- #23 is closed: updating to main's #19 solver fix resolves the symmetric-tube stall. #18 contains
  no #23 changes. Duplicate graph #8 and measurements #15 are also closed; do not re-raise them.
- Corpus #26, codex/corpus-known in /tmp/rulercraft-io: four original closed STL designs and fixed
  JSON answers. Block has 3/5/8 mm holes spaced 20; stepped cylinder diameters 20/12; sphere seat
  radius 12; bracket inner faces 24 apart. Editable source tools/generate_known.py, standard library
  only. Answers are independent of engine output and never rewritten by the generator.
- Last raw result: fmt/clippy/workspace tests/plan render+check and 3 plan tests pass; corpus check
  0.14 s. Every listed dimension, closed boundaries and no non-manifold edges checked. Wrong-value:
  changed small-hole radius 1.5 -> 1.6, check failed at diameter 3.2 vs 3; restored and green.
- Corpus is mathematical designs, not print/scan evidence. Printed/caliper and published-dimension
  downloaded-part checks remain. Registry docs/test-corpus.md; spec docs/specs/test-corpus.md.
- Next: publish #26 ready, inspect live claims and implementation plan. MCP checkpoint is due after
  #18 merges. Claude owns segmentation. Original checkout belongs to Claude; leave it alone.
- Logan merges using merge commits only. Regenerate plan conflicts, never hand-merge. Re-run
  plan.py next immediately before claims. Goal: continue small unclaimed features until usage stops
  the run. The usage helper failed before returning data; do not poll it.
