# RulerCraft

**Measure any mesh, and turn it back into editable CAD, with Claude doing the driving.**

> [!NOTE]
> As of 2026-11-18: **use it inside SolidCraft** — the RulerCraft plugin opens a scan (STL/PLY/OBJ/3MF/glTF/GLB) as an
> editable part, a plate with its outline, holes, bosses, pockets and named sizes, with a report of what it could not rebuild
> ([install and use](docs/install-solidcraft-plugin.md)). The engine also fits planes, spheres and cylinders, finds flat
> and round regions, and measures distances, thickness and deviation, through a CLI and an MCP server. Nothing has
> yet been checked against a real printed or scanned part (ROADMAP R1).
> Progress is tracked in
> [STATUS.md](STATUS.md) and [ROADMAP-PROGRESS.md](ROADMAP-PROGRESS.md).

Download an STL from a model-sharing site, or take a phone scan, and ask: *what diameter are these holes, how far
apart are these faces, rebuild this as a part I can edit.* RulerCraft is the Rust mesh engine that
answers: it reads meshes, finds and fits their planes, holes, cylinders and spheres, and measures
them, over a CLI and an MCP server. The CAD half (building the solid, STEP export) is
[SolidCraft](https://github.com/tangency-labs) and its kernel SolidCore.

The project's locked intent is [INTENT.md](INTENT.md); the competitive scope is
[docs/specs/product-scope.md](docs/specs/product-scope.md).

## Choose a workflow

| Job | SolidCraft plugin | RulerCraft CLI | RulerCraft MCP |
| --- | --- | --- | --- |
| Rebuild a plate-like triangle mesh | Import and edit in SolidCraft | Export an editable SolidCraft design | Use the plugin through SolidCraft MCP, or inspect with RulerCraft MCP |
| Fit and measure selected cloud regions | Desktop inspection is planned | Fit and measure XYZ, PTS, ASC and PLY points | Open, select, fit, measure and calibrate a cloud |
| Rebuild from cloud picks and hand readings | Native integration is planned | Fit and measure for manual assembly | Emit an editable plate with one hole and its measurement evidence |

[Install the tested plugin preview](docs/install-solidcraft-plugin.md) or
[drive an assisted rebuild from MCP](docs/agents-and-solidcraft.md#rebuild-a-plate-from-a-point-cloud).

## Try it

```bash
cargo run -p rulercraft-cli -- info fixtures/known/hole-block.stl
cargo run -p rulercraft-cli -- fit sphere fixtures/known/sphere-seat.stl --near 0 0 12 12.01 --sigma 0
cargo run -p rulercraft-mcp # stdio server; connect using an MCP client
cargo run -p rulercraft-gui -- fixtures/known/hole-block.stl # browser prototype for development
```

The browser prototype (`rulercraft-gui`) runs locally and opens in your browser. See [the GUI page](docs/specs/gui.md).

Project commands are `rulercraft project create`, `project fit` and `project info`; see
[project format and CLI](docs/specs/project-file.md). [MCP tools](docs/specs/mcp.md) provide bounded
selection pages, fits, two-selection measurements, segmentation, thickness, comparison and undo/redo.
MCP also opens point clouds, calibrates scale from a hand-measured span, and builds an editable
rectangular plate with one through or blind hole through `assist.plate`. The result carries the source
of each dimension. Start with the [assisted rebuild walkthrough](docs/agents-and-solidcraft.md#rebuild-a-plate-from-a-point-cloud).
The assisted route has synthetic and SolidCraft host checks. A physical phone export and the desktop
workflow are still open.

## How it is built

- **Checked, not trusted.** Every measurement is tested against answers known independently of the
  code: math with exact answers and parts whose design dimensions are known. See [PROCESS.md](PROCESS.md).
- **Fast to work on.** The whole test suite runs in minutes.
- **Independent.** Written from our own specs ([docs/specs](docs/specs)) and textbook methods; no code
  from other applications and no copyleft code. See [DECISIONS.md](DECISIONS.md).

## Docs

| | |
| --- | --- |
| [CONTEXT.md](CONTEXT.md) | What this is and is not, direction |
| [ROADMAP.md](ROADMAP.md) | Milestones and what each one lets you do |
| [docs/implementation-plan.md](docs/implementation-plan.md) | Build order, parallel lanes, what each PR contains |
| [PROCESS.md](PROCESS.md) | How a feature gets done and proven |
| [AGENTS.md](AGENTS.md) | Rules for AI agents and contributors |
| [CONTRIBUTING.md](CONTRIBUTING.md) | For people who want to help, including with their own AI agent |
| [DECISIONS.md](DECISIONS.md) | Why things are the way they are |

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
