# Distribution

Status: *Plan (2026-11-07)*. Nothing is published without Logan's yes at the time (AGENTS.md). This page is the plan so
that, when he says yes, the steps are known.

## What ships

| Piece | Who uses it | Channel |
| --- | --- | --- |
| `rulercraft-mesh` (engine) | programs, SolidCraft | crates.io (and git) |
| `rulercraft` (CLI) | scripts, CI | GitHub Releases binaries; `cargo install` |
| `rulercraft-mcp` (MCP server) | Claude and other agents | GitHub Releases; install line for Claude Code/Desktop in the README |
| RulerCraft app (native, D-012) | people | GitHub Releases installers; later Flathub, winget, Homebrew |
| Web build (later) | people with nothing installed | static hosting (for example GitHub Pages) |
| SolidCraft plugin (if their design lands) | SolidCraft users | whatever SolidCraft decides (#1221) |

## Platforms

Linux x86_64 and aarch64 (AppImage and .tar.gz; Flatpak later), Windows x86_64 (.zip, then an installer), macOS
universal (.dmg). Built by CI from a tag.

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
