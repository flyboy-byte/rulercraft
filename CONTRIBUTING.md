# Contributing

For people (not agents). Everything is a pull request; Logan reviews and merges, so you cannot
break `main` by following this page.

## Set up

- Rust 1.99 or newer (`cargo`, `rustfmt`, `clippy`). Python 3 for the plan tool.
- `cargo test --workspace` should pass in a few minutes on a fresh clone.

## Pick something

`python3 tools/plan.py next` prints what is ready and unclaimed. Claim it with a draft PR whose
title starts with the feature id, for example `fit.sphere: least-squares sphere fit`. Then follow
PROCESS.md "How a feature gets done". The PR description is your report (template in PROCESS.md).

## Bring your own agent (Claude, Codex or another)

If you want to put your own AI credits toward RulerCraft, this is all it takes:

1. Fork `flyboy-byte/rulercraft` and clone your fork (or ask Logan for collaborator access).
2. Start a session in the clone and paste:

   ```
   You're contributing to RulerCraft (an independent Rust mesh engine, MIT OR Apache-2.0).
   Read AGENTS.md first (it outranks everything), then PROCESS.md, docs/architecture.md and
   HANDOFF.md. Run `python3 tools/plan.py next` against flyboy-byte/rulercraft (it lists ready
   features and every claim from open PRs). Take the first ready, unclaimed feature: open a draft PR
   to flyboy-byte/rulercraft titled "<feature id>: ..." before writing code, set owner to
   "<your name>-<model>" in its plan/features file, follow the spec and PROCESS.md (math checks with
   known answers, one wrong-value run, tests in seconds), then mark the PR ready with the report
   from PROCESS.md. Keep it light: no new process, checks or dependencies.
   ```

3. Logan (or someone he trusts) reviews lightly and merges. One feature per PR keeps that fast.

What helps most: any feature `tools/plan.py next` lists as ready. The specs in `docs/specs/` say what
each must do, so a fresh session needs nothing else. Cost guard: a feature should take an agent an hour
or two, not a day; if it is growing past that, say so in the PR instead of pushing on.

## Rules that matter

- **Licence.** Your contribution is licensed MIT OR Apache-2.0 (see LICENSE-MIT, LICENSE-APACHE).
  Do not copy code from other applications or from GPL/LGPL/AGPL projects. Textbook algorithms and
  file-format standards are fine.
- **No outside names.** Do not name other products in code, docs, commits or PRs, even ones that
  inspired an idea (AGENTS.md "Independence").
- **Proof.** A user-facing number needs a math check and, where possible, a known-design or
  reference check (PROCESS.md). Never loosen a tolerance to make a test pass.
- **Speed.** Keep your feature's tests under a minute.

Unless you state otherwise, any contribution you submit is dual-licensed MIT OR Apache-2.0 as
above, without additional terms.
