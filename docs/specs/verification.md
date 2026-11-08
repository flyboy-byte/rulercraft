# Verification at scale

Status: *Plan (2026-11-07, D-013)*. Serves INTENT.md principle 1 (a wrong number is worse than a missing one).

Logan will not use RulerCraft hard enough to break it the way it needs breaking, and he cannot check geometry code by
eye. So machines and agents do the breaking, and every break becomes a permanent test. This page says how.

## Oracles: how a test knows the right answer without another program

1. **Known answers**: parametric shapes generated with exact dimensions (the *shape zoo*: planes, spheres, cylinders,
   cones, tori, boxes, filleted and drilled parts) at many sizes, densities, positions, noise levels, hole and outlier
   rates. The generator is test-only code.
2. **Invariance**: moving, turning, rescaling, reordering vertices or triangles, or flipping winding where it must not
   matter changes no answer beyond a stated tolerance.
3. **Round trips**: write then read (project files, recipe JSON), fit then sample then refit.
4. **Agreement between faces**: the library, CLI, MCP server and app give the same numbers for the same request; later
   native and Wasm, CPU and GPU.
5. **Designed parts and real measurements**: our committed designs, Logan's printed parts with caliper numbers, and
   published dimensions (kept local, D-004).

## Tiers

| Tier | When | Budget | What runs |
| --- | --- | --- | --- |
| **T0 PR gate** | every PR | under 5 min | unit, math, known design, invariance, hostile input; the author's wrong-value run (PROCESS.md) |
| **T1 Nightly** | scheduled CI | under 30 min | property tests over the shape zoo with many seeds; the regression corpus compared to stored answers within tolerance; face agreement (library vs CLI vs MCP vs app server); benchmarks against a stored baseline |
| **T2 Weekly and before a release** | scheduled or by hand | hours | fuzzing every reader and the command layer; mutation testing (every surviving mutation becomes a test or the rule is deleted); soak runs on large local scans; a break-it session |
| **T3 Real world** | ongoing | — | Logan's parts, SolidCraft users' reports through the "a number looks wrong" issue form |

T0 stays fast (D-003). T1 and T2 are where the volume goes, so they never slow a PR.

## Regression corpus

`fixtures/regress/`: input meshes (generated or our own designs) with stored expected numbers and the tolerance and
reason for each. A script runs every face over it and reports differences. Expected numbers come from the known
answer, never from RulerCraft's own earlier output, unless the case is explicitly a "did it change" guard and says so.
Every confirmed bug adds a case.

## Mutation testing

The house rule "every rule needs a wrong-value run that a test fails on" is done by hand today. A mutation tool does it
automatically across the whole engine: it changes operators and constants and reports changes no test notices. Each
survivor is either a missing test (add it) or a rule that does nothing (delete it).

## Break-it sessions

Any session can be a break-it session when credits allow. Prompt:

```
You are trying to break RulerCraft. Read AGENTS.md and docs/specs/verification.md. Your only goal is to make it give a
wrong number, hang, use unbounded memory, or crash, through the library, CLI, MCP server or app, with inputs a
real user or a hostile file could produce. For each break: a minimal reproduction, the right answer and how you know
it, then a PR titled "test: ..." adding a failing test, or "fix: ..." with the fix and the test. Do not loosen any
tolerance. Report what you tried that did not break, too.
```

## Measures we track

Shapes in the zoo, corpus cases, fuzz hours per reader, mutation score of the engine, faces covered by agreement
tests, benchmark baselines. Reported in STATUS.md once the tools exist.

## Tools

Mutation testing and fuzzing run as developer tools installed on the machine or CI runner, not as dependencies of the
shipped crates. A fuzz harness that needs a library dependency lives in its own crate outside the workspace and needs
Logan's yes (AGENTS.md). Scheduled CI workflows are a hot file change: one PR at a time.

## First shape-zoo slice

The first fast, deterministic slice samples full spheres and open cylindrical surfaces at multiple radii and
point densities. Each point is generated from an analytic centre, radius and (for cylinders) height; the tests
compare fitted dimensions with those independent inputs, allowing only a stated allowance for f32 quantization.
Later slices add more shapes, partial surfaces, noise, outliers and seeded sweeps. A slice is not the whole zoo.

The partial-surface slice adds 60-degree sphere caps and 120-degree cylinder arcs at three radii and three
deterministic noise seeds, with radial noise bounded to 0.1% of radius. The fit allowance is 1% of radius.
