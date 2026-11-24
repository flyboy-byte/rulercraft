# Handoff

Last updated 2026-11-22, Codex. This records the next milestone after the public 0.1.0 release and the comparison with SolidCraft 0.46.

## Paused (2026-11-24)
Development is paused (D-017): Logan prefers scan-to-CAD as a SolidCraft feature and is helping that elsewhere. The
code is as of #129. Read D-017 before resuming. Still open for Logan: push the plugin-v0.1.0 tag; Codex PRs #127 and
#128.

## Read first

Read [INTENT.md](INTENT.md), [AGENTS.md](AGENTS.md), and [PROCESS.md](PROCESS.md). Then read the
[next milestone and phone trial](docs/assisted-rebuild-next.md) and the
[assisted rebuild contract](docs/specs/assisted-rebuild.md).

## Where things stand

- [Plugin 0.1.0](https://github.com/flyboy-byte/rulercraft/releases/tag/plugin-v0.1.0) is published at
  `e6b6fba`. The 16-case host gate passed. I downloaded the public ZIP, verified its checksum
  and source commit, and installed and edited the known hole block through the Linux host.
  Logan confirmed installation and the **Model › Plugins group › RulerCraft** entry on desktop
  before the release. An earlier preview may also display 0.1.0. The public package has **Scan units › Auto**.
  [Install guide](docs/install-solidcraft-plugin.md)
- The plugin and CLI share the plate rebuild engine. It recognizes straight or circular outlines,
  holes, pockets, bosses, steps, windows, and supported chamfers and fillets. It creates named
  dimensions and reports omitted or simplified features, sampled deviation, and host volume.
  The plugin reads supported OBJ units headers in Auto. CLI and RulerCraft MCP have explicit units.
  [Support matrix](docs/capabilities.md)
- RulerCraft MCP opens and calibrates a cloud, returns picked coordinates, and uses `assist.plate`
  to draft a rectangular plate with one hole. It reports dimension sources. Synthetic face fits
  reduced the worst width error from 0.676 to 0.396 mm across 12 captures. A draft from a noisy
  cloud matched the analytic volume of its emitted dimensions before and after a thickness edit.
  Some noisy quarter-wall fits passed the coverage guard. Mixed off-wall samples can inflate
  fitted coverage. [Assisted trials](docs/specs/assisted-rebuild.md)
- The faceted tree reported by Logan opened as a mesh. The plate rebuild originally matched only
  3 of 281 sampled vertices within tolerance. #149 makes gross mismatches refuse and has an
  original synthetic regression. The supplied tree stayed outside the repo. The manually built
  tree project was a separate demonstration. Automatic branch reconstruction remains planned.
- SolidCraft 0.46 already opens E57, measures its points, and places datums. Native Convert Mesh
  handles closed meshes. On the locally checked 0.45.0-rc.4 host, Convert Mesh built the known hole
  block and bracket with a conversion feature and zero named user parameters. Focus this milestone
  on turning a rough capture and hand readings into editable design intent.
  [Host comparison](docs/assisted-rebuild-next.md#what-solidcraft-already-covers)

## Next

1. Run `python3 tools/plan.py next`, check current claims, and claim `sc.assisted` with a draft PR.
   Prove current host access to selected scan evidence and durable dimension evidence first.
   Treat the historical 8,192-vertex `model.mesh` limit as something to retest on the target host.
2. Carry the existing assisted plate behavior into the SolidCraft workflow. Use a generated capture
   and explicit selections first. Verify plugin and MCP agreement, save and reopen the design,
   and edit thickness and diameter. Preserve evidence and prompt for underdetermined geometry.
3. Run `verify.phone` when a measured capture arrives. Compare the native and assisted paths on
   the same original export and hold out width. Record refusal and editing effort as well as
   dimensional error. [Trial protocol](docs/assisted-rebuild-next.md#first-physical-phone-trial)

Keep the plugin strategy until a specific host capability blocks a demonstrated benefit. Capture a
small reproduction before considering an upstream PR. Do the synthetic milestone while waiting
on a scan. Broader meshing, independent E57 import, and faceted and branch reconstruction are still
tracked in the [roadmap](ROADMAP.md).

## Owed and waiting on Logan

- One unmodified phone export of a real plate or bracket with one round hole and a measured
  length. Width, thickness, diameter, through status or blind depth, and the import report will
  make the comparison stronger. [What to send](docs/assisted-rebuild-next.md#what-logan-can-send)
- Wider host-version and platform acceptance remains open. Any new publishing or upstream
  outreach needs Logan's approval under AGENTS.md.

## Verification notes

- `scan_like.rs` covers noisy turned meshes. Exact fixtures and SolidCraft's host package tests
  remain the backstop for plate rebuild changes. See the earlier
  [assessment](docs/findings/2026-11-19-codex-assessment.md) and
  [readiness checks](docs/findings/2026-11-21-current-state.md).
- Reader and fit-command fuzzing run in the gate. Longer fuzz passes are explicit. The
  `ply_memory` child memory cap has flaked once under parallel load.
- Use a separate worktree. Keep supplied models and measurements in `fixtures/private/` or
  `local/`. Use the plan generator for STATUS.md and ROADMAP-PROGRESS.md.
