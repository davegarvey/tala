## Why

Tala skills currently carry a skill-document version, but do not identify the
CLI version that produced them or the minimum CLI version they require. An
agent can therefore follow stale instructions after the binary has been
upgraded, including commands that have been renamed or removed. The skill and
CLI need an explicit, machine-readable compatibility contract.

## What Changes

- Add explicit Tala CLI compatibility metadata to the embedded skill documents.
- Keep skill-content versioning separate from CLI versioning.
- Record both the minimum supported CLI version and the CLI version that
  generated the installed skill files.
- Update the skill instructions to compare the metadata with `tala --version`,
  treating the binary as authoritative.
- Make `tala init` install versioned skill metadata consistently for existing
  and new projects without changing project identity configuration.
- Add validation and end-to-end coverage for metadata shape, version comparison,
  stale-skill guidance, and generated-document consistency.

## Capabilities

### New Capabilities

- `skill-cli-compatibility`: Machine-readable skill metadata and agent guidance
  for detecting CLI version incompatibility and stale generated instructions.

### Modified Capabilities

- `openspec/specs/project-setup/spec.md`: Skill and command files installed by
  `tala init` must contain the CLI compatibility metadata and current generation
  information.

## Impact

- Affected source: CLI initialization and embedded OpenCode skill/command
  documents.
- Affected tests: initialization, documentation consistency, and CLI version
  compatibility tests.
- No wire-protocol or daemon API changes are required.
- No new runtime dependencies are expected.
