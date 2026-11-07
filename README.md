# RulerCraft

**Measure any mesh, and turn it back into editable CAD, with Claude doing the driving.**

> [!NOTE]
> Early days (started 2026-11-06). Today it reads STL and prints a summary. Progress is tracked in
> [STATUS.md](STATUS.md) and [ROADMAP-PROGRESS.md](ROADMAP-PROGRESS.md).

Download an STL from a model-sharing site, or take a phone scan, and ask: *what diameter are these holes, how far
apart are these faces, rebuild this as a part I can edit.* RulerCraft is the Rust mesh engine that
answers: it reads meshes, finds and fits their planes, holes, cylinders and spheres, and measures
them, over a CLI and an MCP server. The CAD half (building the solid, STEP export) is
[SolidCraft](https://github.com/tangency-labs) and its kernel SolidCore.

## Try it

```bash
cargo run -p rulercraft-cli -- info part.stl
```

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
| [ROADMAP.md](ROADMAP.md) | Milestones R1-R6 and what each one lets you do |
| [PROCESS.md](PROCESS.md) | How a feature gets done and proven |
| [AGENTS.md](AGENTS.md) | Rules for AI agents and contributors |
| [CONTRIBUTING.md](CONTRIBUTING.md) | For people who want to help, including with their own AI agent |
| [DECISIONS.md](DECISIONS.md) | Why things are the way they are |

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
