# Contributing

For people (not agents). Everything is a pull request; Logan reviews and merges, so you cannot
break `main` by following this page.

## Set up

- Rust 1.99 or newer (`cargo`, `rustfmt`, `clippy`). Python 3 for the plan tool.
- `cargo test --workspace` should pass in a few minutes on a fresh clone.

## Pick something

Anything that makes RulerCraft better is welcome; the roadmap is a map, not a limit. docs/where-to-help.md lists the
standing areas (break it, accuracy, speed, everyday use, inputs, interfaces, the CAD loop, docs) and how to choose.
`python3 tools/plan.py needs` shows what needs help right now. Claim with a draft PR: a roadmap feature uses its id in
the title (`fit.sphere: ...`), anything else an area tag (`fix: ...`, `improve: ...`). Then follow PROCESS.md. The PR
description is your report (template in PROCESS.md). Found a number you do not believe? Use the "A number looks
wrong" issue form; those reports are some of the most valuable help there is.

## Bring your own agent (Claude, Codex or another)

If you want to put your own AI credits toward RulerCraft, this is all it takes:

1. Fork `flyboy-byte/rulercraft` and clone your fork (or ask Logan for collaborator access).
2. Start a session in the clone and paste:

   ```
   You're contributing to RulerCraft (an independent Rust mesh engine, MIT OR Apache-2.0).
   Read AGENTS.md first (it outranks everything), then PROCESS.md, docs/architecture.md and
   HANDOFF.md and docs/where-to-help.md. Run `python3 tools/plan.py needs` against flyboy-byte/rulercraft.
   Pick the one thing that helps the app most for its size (a roadmap feature or any improvement), open a
   draft PR to flyboy-byte/rulercraft before writing code (feature id or area tag in the title), follow the
   spec and PROCESS.md (known answers, one wrong-value run, tests in seconds), then mark the PR ready with the
   report from PROCESS.md. Keep it light: no new process, checks or dependencies.
   ```

3. Logan (or someone he trusts) reviews lightly and merges. One feature per PR keeps that fast.

What helps most: whatever `tools/plan.py needs` shows, and break-it sessions (docs/specs/verification.md). The specs in `docs/specs/` say what
each must do, so a fresh session needs nothing else. Cost guard: a feature should take an agent an hour
or two, not a day; if it is growing past that, say so in the PR instead of pushing on.

Shipping and releases: docs/distribution.md. User-visible changes get a line in CHANGELOG.md.

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
