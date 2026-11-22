# Distribution

Status updated 2026-11-22. [SolidCraft plugin 0.1.0](https://github.com/flyboy-byte/rulercraft/releases/tag/plugin-v0.1.0)
is published. The public ZIP was downloaded, checksum-verified, installed and exercised on the known
hole block. Further publishing requires Logan’s explicit approval under AGENTS.md.

## What ships

| Piece | Who uses it | Channel |
| --- | --- | --- |
| `rulercraft-mesh` (engine) | programs, SolidCraft | crates.io (and git) |
| `rulercraft` (CLI) | scripts, CI | GitHub Releases binaries; `cargo install` |
| `rulercraft-mcp` (MCP server) | Claude and other agents | GitHub Releases; install line for Claude Code/Desktop in the README |
| Web build (later) | people with nothing installed | static hosting (for example GitHub Pages) |
| SolidCraft plugin | SolidCraft desktop users | Tested GitHub Actions previews and tagged GitHub releases |

## Plugin delivery

The plugin is one WebAssembly component ZIP. CI checks it against SolidCraft CLI 0.45.0-rc.4 on
Linux x86_64, runs the host fixture suite, installs the actual ZIP, and checks a known part before
and after a thickness edit. Changes to the mesh engine also trigger packaging. Download the preview
from a successful run on `main` and follow the [install guide](install-solidcraft-plugin.md).
Other desktop platforms and newer host versions remain compatibility checks.

Once Logan approves a release, push the version tag matching `plugin.json`, such as
`plugin-v0.1.0`. The release job takes the ZIP from the same run that tested it and verifies the ZIP
checksum and source commit before attaching it to the public release. Rust binary distribution and
crates.io publishing are separate work.

## Versions and changes

SemVer. 0.x until v1 (docs/v1.md); the engine's public API is listed in docs/specs/library-api.md. Every PR that changes
what a user sees adds a line under "Unreleased" in CHANGELOG.md.

## Release checklist

1. T0 and T1 green on the release commit; T2 run since the last release (docs/specs/verification.md).
2. Version bumped, CHANGELOG.md "Unreleased" moved under the version and date.
3. Third-party licence list regenerated; NOTICE current (SolidCraft theme, D-011).
4. Logan creates the tag. CI builds every artifact.
5. Smoke test each artifact on its platform: open a known part, fit, measure, compare with the stored numbers.
6. Logan publishes the release (and crates.io).

## Decisions waiting for Logan (cost or identity)

- Code signing: macOS needs a paid Apple developer account; Windows signing needs a certificate. Without them,
  installers show warnings.
- A name check on "RulerCraft" before the first public release.
- Private security reports: GitHub's private vulnerability reporting is a repository setting.
- Update checks: none by default (local first, no telemetry); an opt-in check could be added later.
