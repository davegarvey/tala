# Proposal

## Why

Issue #155 reports that bare `tala guide` loads roughly 19,000 characters into each new agent's context, including a harness implementation most agents never need. Agents need a short first exchange and essential rules, then command help and focused topics as questions arise.

## What Changes

- **BREAKING**: Bare `tala guide` returns a quickstart under 2,000 Unicode characters instead of the full manual.
- **BREAKING**: Successful guide output defaults to plain Markdown; explicit `--format=json` retains the standard envelope. Other commands and default structured usage errors remain unchanged.
- Add `tala guide all` for the complete reference and `tala guide harness` for the supervisor recipe, removed from `listening`.
- Point top-level help to the quickstart, list topics with short descriptions, and defer command syntax details to `tala <command> --help`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-cli`: Guide content selection, discoverability and the guide-specific default output format.

## Impact

Changes affect `src/integration.rs`, CLI parsing/render selection, `docs/agent-guide.md`, help text and existing guide tests. Scripts parsing bare guide output must request `--format=json`; callers wanting the previous full content must also select `all`. Use a breaking conventional commit for release automation. No daemon protocol, database, dependency or messaging behaviour changes are needed.
