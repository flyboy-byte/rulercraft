# Where to help

The roadmap (ROADMAP.md, plan/features/) is a map, not a cage. RulerCraft is one person plus whatever agents and
people show up, so the best work is often not the next roadmap item but whatever makes the app better right now.
Anything that serves INTENT.md is welcome. You do not need permission to improve something; you need a PR that says
what it does and what proves it.

## "Find something to work on"

When a session is told only that, it does this, in about five minutes:

1. `python3 tools/plan.py needs`: ready features, half-finished ones and what they still lack, open findings,
   ignored tests, open PRs (so you do not collide).
2. Run the thing: `cargo run -p rulercraft-cli -- info fixtures/known/hole-block.stl`, the MCP server, the app.
   Notice what is rough: a confusing message, a slow step, a missing option, a number you would not trust.
3. Pick **one** thing that would help the app most for its size, tell Logan in one line what you picked and why,
   and start. Prefer, roughly in this order: something wrong (a bad number, a crash), something a user hits every
   time, something that unblocks other work, something on the roadmap, something new and cool.
4. Claim it with a draft PR (see AGENTS.md "Crosstalk"). A roadmap feature uses its id in the title; anything else
   uses an area tag: `fix: ...`, `improve: ...`, `perf: ...`, `test: ...`, `docs: ...`, `idea: ...`.

You do not have to follow the phase order in docs/v1.md. If something off-plan is clearly worth doing, do it and say
why in the PR; add a feature file only when it is a capability a user would name.

## Standing areas (always open)

| Area | Examples |
| --- | --- |
| **Break it** | make the engine give a wrong number, hang, or crash; every confirmed break becomes a regression test (docs/specs/verification.md) |
| **Accuracy** | tighter known-answer tests, new shapes in the test zoo, honest uncertainty in results |
| **Speed** | profiling big meshes, the shared spatial index, threads, GPU paths checked against the CPU |
| **Everyday use** | clearer messages, better defaults, fewer clicks in the app, CLI and MCP options that match each other |
| **Inputs** | more file types, point clouds, scanner exports, phone and photo sources (docs/specs/scan-sources.md) |
| **Interfaces** | every capability reachable from the library, CLI, MCP and app with the same numbers |
| **CAD loop** | the recipe, SolidCraft hand-off, checks against the rebuilt part |
| **Docs** | a page a newcomer needs, a spec that is vague, a stale paragraph |

## How to judge if it is worth it

- Does it serve INTENT.md (trustworthy numbers, the four jobs of docs/v1.md, local, permissive, joinable with SolidCraft)?
- Can it be proven cheaply (a known answer, a wrong-value run that fails)?
- Is it sized for one PR (an agent's hour or two)? If bigger, land the first useful slice and write the rest in HANDOFF.md.

What still needs Logan: anything in AGENTS.md "Never without Logan's explicit say-so", and choices that change what a
user sees in a way he would care about. Everything else: pick a reasonable default, write it down, keep going.
