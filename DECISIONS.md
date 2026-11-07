# Decisions

Numbered, never rewritten. A later decision that changes an earlier one says `supersedes D-00N`.

## D-001 Independent, MIT OR Apache-2.0 (2026-11-06)

**Decision.** RulerCraft is new, independent code, licensed MIT OR Apache-2.0. It contains no code,
text, messages, data or structure taken from another application, and the repository does not name
other applications it might be compared with or inspired by (AGENTS.md "Independence").

**Why.** The aim is the Tangency Labs ecosystem (SolidCraft/SolidCore, MIT/Apache). An earlier attempt
ported a copyleft application line by line; that work can never be relicensed and cannot be used here.
What carries over is knowledge of how a mesh-measurement tool should behave, written fresh as specs.

**Rules that follow.**
- Specs (docs/specs/) describe behaviour in our own words: inputs, outputs, meaning, edge cases, units.
- Implementations come from the specs, published file-format standards and textbook methods.
- An idea from outside is welcome, but it enters the repository only as our own description of the
  behaviour: no name, link, quote or credit to the outside source, in any file, comment, commit
  message, PR or issue.
- Claude and Codex may have seen other applications' sources, so this is independence in practice, not
  a strict legal clean room. The safeguards are these rules and review for transliteration. If RulerCraft is ever commercialised, get one legal consult first.

## D-002 SolidCore and SolidCraft do the CAD half (2026-11-06)

**Decision.** RulerCraft is the mesh side only. Building solids, STEP export and projecting points onto
CAD surfaces go through SolidCraft (its MCP server first) and later SolidCore (Apache-2.0) directly,
behind an optional cargo feature.

**Why.** SolidCore is a B-rep/NURBS kernel with no mesh reading, mesh connectivity or robust primitive
fitting; RulerCraft fills exactly that gap. SolidCore is prerelease, changes fast and is heavy to build,
so the core crates never depend on it.

## D-003 Verification budget (2026-11-06)

**Decision.** Each feature is checked by, in this order: math checks with known answers; parts with known
design dimensions; where available, a few local reference numbers compared within a stated tolerance;
one deliberately wrong value that must fail. Not byte-identical, not whole-state traces. A feature's tests
run in under a minute; the whole suite in under five. Fuzzing and large sweeps only before a release,
when Logan asks.

**Why.** Demanding byte-identical results and full internal-state comparison after every step made earlier
work trustworthy but very slow (about a day per feature). Users see numbers; numbers that agree with an
independent known answer within tolerance are what matters.

## D-004 Outside reference numbers stay local (2026-11-06)

**Decision.** Numbers from any other program, and models downloaded from the web, are kept in gitignored
`fixtures/private/` with their notes in gitignored `local/`. Committed tests use only generated meshes,
our own designs, and math. A test that needs local data skips when it is absent and says so.

**Why.** It keeps outside products out of the repository and its history (D-001), and keeps third-party
model licences out of our distribution.

## D-005 Logan's account first, public (2026-11-06)

**Decision.** The repository is flyboy-byte/rulercraft, public from the start. Moving it to tangency-labs
later is the maintainers' and Logan's call; nothing here depends on it.

## D-006 Default dependencies (2026-11-06)

**Decision.** No dependency by default. Pre-approved when a feature needs them: `serde`/`serde_json`, a
zip/deflate reader (3MF), the official Rust MCP SDK. Each still needs an MIT/Apache/BSD-style licence,
checked and stated in the PR. Linear algebra stays in our own small `math/` module for R1.

**Why.** Fresh sessions should not stall asking about obvious choices, and a light dependency tree keeps
builds and CI fast.

## D-007 — MCP transport dependencies (2026-11-07)

Use the pre-approved official Rust MCP SDK `rmcp` (Apache-2.0) with stdio/server/macros/schema
features, plus its Tokio runtime (MIT), Serde (MIT OR Apache-2.0) and existing serde_json
(MIT OR Apache-2.0). No network transport is enabled. Engine crates remain independent of MCP.

## D-008 Project JSON serialization (2026-11-07)

**Decision.** `project.file` uses serde (derive) and serde_json (float_roundtrip), both
MIT OR Apache-2.0, pre-approved by D-006. Source bytes are embedded in version 1 JSON, avoiding a new
hashing or archive dependency. Exact float round trips keep unchanged cached fits from being reported
as changed solely by JSON parsing.

## D-009 3MF archive reader (2026-11-07)

**Decision.** `io.3mf` uses zip 2.6 (MIT), default features off with deflate enabled, as pre-approved by
D-006. Stored/deflated model and relationship parts are read in memory with byte caps, never extracted.
The core geometry XML subset is implemented here without another dependency. zip's Rust 1.73 minimum
fits the workspace's declared 1.85 minimum; no minimum-version change.
