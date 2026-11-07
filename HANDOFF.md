# Handoff

Last updated 2026-11-07, Codex.

- `io.ply`: verified, PR #2, branch `codex/io-ply`, isolated worktree `/tmp/rulercraft-io`.
  ASCII / binary LE / binary BE through MeshBuilder; extension dispatcher and CLI info.
  Last raw result: fmt and clippy pass; 11 tests pass; plan render/check pass (28 features).
  Wrong-value run: fan third corner changed from k+1 to k, cube check failed; restored and green.
- Next: `io.obj`, a separate PR stacked on #2 until Logan merges PLY. Follow mesh-import spec.
- Claude owns `math.core`, PR #1; original checkout has its uncommitted math work. Do not touch it.
- Logan merges with merge commits. After merges regenerate STATUS.md / ROADMAP-PROGRESS.md;
  never hand-merge generated files. Retarget stacked OBJ PR to main once #2 merges.
- Known-design corpus checks remain pending `corpus.known`; no dependencies added.
