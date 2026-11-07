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
