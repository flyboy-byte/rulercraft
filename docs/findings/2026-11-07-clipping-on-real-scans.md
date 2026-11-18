# Sigma clipping does not settle on a real scan (2026-11-07)

**Seen.** On a private real scan (412,755 vertices), the base face (5,969 vertices) fitted as a plane:

| Cut-off | Points used | sigma (mm) | form error (mm) | rounds |
| --- | ---: | ---: | ---: | ---: |
| none | 5,969 | 0.0931 | 0.917 | 1 |
| 3 sigma | 5,469 | 0.0603 | 0.368 | 10 (the cap) |

With no cut-off the result agrees with an earlier independent implementation's number for the same
selection (0.093). With 3 sigma the loop never stops changing the kept set: each round trims a few more
tail points, because a real surface has heavier tails than noise on a perfect plane. It stops at the
10-round cap, so the reported sigma depends on the cap. The normal moves by under 1e-4 between them.

**Why it matters.** The flatness a user reads with clipping on is partly an artefact of the round limit.

**Options (not decided).** (a) Clip once against the all-points fit's sigma (one round, no creep);
(b) iterate but keep the cut-off fixed at k times the first sigma; (c) keep the loop and report "did not
settle" when the cap is hit. (b) is the usual fix and keeps the spec's intent. Decide before `fit.robust`
(R3), which reworks this area anyway; until then the spec and code are unchanged.

## 2026-11-07 reply, Codex / checkpoint.r1-exit #32

Reproduced on current integrated release: the same 5,969-point base fit with sigma=3 uses 5,469 points
and reaches round 10. No clipping semantics changed in this audit. This remains an open decision for
fit.robust; treat capped clipping quality statistics as dependent on the round limit. The sigma=0
base result is 0.0930818265 mm and CLI/MCP agree. This does not establish real dimensional accuracy.

## 2026-11-12 reply, Claude: decided, option (b)

The cut-off is now k times the all-points fit's sigma, fixed for every round, in the plane, sphere and
cylinder fits (fit-plane.md, fit-cylinder.md). New tests in `tests/clip_settles.rs` use Laplace
(heavy-tailed) residuals: on 7,200 mirrored plane points the old loop kept 88 points at k = 1 (5,438
is right, counted without a fit), and it overshot on the sphere and cylinder too; all now settle in
two or three rounds with the kept set the rule defines. The private scan's base face was not
re-run in this session (no private fixtures here); expect more than 5,469 points used at 3 sigma.
Left open only for that re-run; close it with the numbers.

## 2026-11-18 reply, Claude: real-scan check of #101

The original private scan is gone, so the fix was checked on two public real scans (a rocker-arm part and a
statue's pedestal from a widely used research scan collection; rescaled to unit size, used locally, not committed).
Planes selected by box and facing, `fit plane --sigma k`, the release binary before #101 against main after it:

| face (points) | k | before: used / rounds | after: used / rounds |
| --- | --- | --- | --- |
| part side A (326) | 1 | 6 / 10 | 313 / 3 |
| part side A | 2 | 241 / 10 | 313 / 2 |
| part side B (351) | 1 | 6 / 10 | 293 / 4 |
| part side B | 2 | 260 / 10 | 293 / 8 |
| pedestal top (1,393) | 1 | 28 / 10 | 1,346 / 4 |
| pedestal top | 2 | 1,014 / 10 | 1,376 / 2 |
| pedestal top | 3 | 1,348 / 5 | 1,379 / 2 |
| pedestal underside (1,033) | 1 | 30 / 10 | 717 / 10 |
| pedestal underside | 2 | 717 / 10 | 894 / 5 |

The other 3-sigma rows did not change. The creep is gone: before, 1-sigma clipping on a real face kept 2 to 3
percent of it. One case still reaches the round cap: the rough pedestal underside at k = 1 (717 kept). With a
fixed cut-off that means the kept set cycles as the plane moves, not creep. Still open for that: stop on a
repeated kept set, or report that the fit did not settle (option c above).
