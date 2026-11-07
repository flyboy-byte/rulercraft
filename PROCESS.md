# Process

How work gets done here, kept fast on purpose. Owner decisions behind it: DECISIONS.md D-003.

## Speed rules

1. **One fast gate.** `cargo fmt --check`, `clippy -D warnings`, `cargo test`, `tools/plan.py check`.
   The whole gate runs in under 5 minutes locally and in CI. Any test that would push past that is
   marked `#[ignore]` and run by hand or at a release gate.
2. **A feature's checks take under a minute** and fit in its own test file.
3. **Verification may not outgrow the code.** If proving a feature is taking longer than writing it,
   stop and ask Logan. Write the question in the PR, not a bigger test.
4. **Land work, don't hoard it.** Tested code goes into a public PR the same day.
5. **No waiting on CI to think.** Run the gate locally, push, and keep working on the next thing while
   hosted CI runs. A red hosted run is read in full before anything else.

## How a feature gets done

A feature is one capability a user would name ("fit a cylinder", "read PLY", "deviation map"). Its plan
file is `plan/features/<id>.json`; its behaviour is `docs/specs/<spec>.md`.

1. **Pick and claim.** `python3 tools/plan.py next`, then a draft PR titled with the feature id, and set
   `owner`, `pr` and `status: "in-progress"` in the feature file.
2. **Spec first.** If the spec is missing or vague, write or fix it in the same PR: inputs, outputs,
   units, edge cases, refusals and their messages. Behaviour only (AGENTS.md clean-room rules).
3. **Checks first.** Write the tests before the code (see "What proves a feature" below).
4. **Build it** from the spec and textbook methods.
5. **Prove the checks bite:** change one value in the code on purpose, watch a check fail, change it
   back. Say in the PR which value you changed.
6. **Record:** set `status` and `verified_by` in the feature file, run `python3 tools/plan.py render`,
   commit the regenerated STATUS.md and ROADMAP-PROGRESS.md. Never hand-edit generated files.
7. **Open the PR for review** with the report below. Logan merges.

## What proves a feature (cheapest first)

| Check | What it is | When |
| --- | --- | --- |
| **Math** | Synthetic input with a known answer: a cylinder of radius 10 fits as 10, a cube's faces fit as its planes. | Always. |
| **Known design** | A part whose design dimensions are known: one we design (committed), or a downloaded model with published dimensions (kept local), or a part Logan printed and measured. | Every user-facing measurement. |
| **Local reference** | Another program's number for the same mesh, kept in gitignored `fixtures/private/` (D-004), compared within a stated tolerance; the test skips when absent. | Optional, when one is available locally. |
| **Wrong value** | One deliberate break that must fail a check. | Always, once, by hand. |
| **Fuzz / sweep** | Random or bulk inputs. | Only at release gates, when Logan asks. |

Tolerances are stated in the test with a reason and never loosened to get green. A reference number is
never replaced with RulerCraft's own output. PR reports and commit messages follow AGENTS.md
"Independence": no outside product names.

## The PR report

The PR body is the report Logan reads instead of the code. Plain words, this shape:

```
## What it does
One or two sentences a user would understand.

## What proves it
- Math: ...
- Known design / local reference: ... (tolerance and why; never name the other program)
- Wrong value: changed X, check Y failed.

## Not proven yet
What is untested or unknown, honestly.

## Commands run
fmt / clippy / test / plan check: pass (times).
```

## Reviews and merging

- Cross-review: when Claude opens a PR, Codex may review it, and the other way round. Findings are
  checked before they are acted on.
- Logan merges, always with a merge commit (no squash, no rebase of pushed branches). Merge `main` into a
  branch to update it.
- After a merge, retarget any stacked PR to `main` and regenerate plan files if they conflict (run
  `tools/plan.py render`, never hand-merge them).

## Ending a session

Commit or push the best state, rewrite HANDOFF.md if anything is left open, and give Logan a paste-back
block: what shipped, what is open, what to check first, what not to re-raise.
