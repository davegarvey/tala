# Spec Delta

## ADDED Requirements

### Requirement: Status reports selected identity context
`tala status` SHALL identify the selector source and report the selected identity and registered project context when available, without exposing private selector data.

#### Scenario: Status reports an explicit selector
- **WHEN** status runs with `--agent` or `--agent-file`
- **THEN** its JSON data identifies the selector source and, when resolvable, includes the public agent ID, display name, and registered project root
- **AND** it does not include the handle, selector contents, or selector-file path

#### Scenario: Status reports an environment selector
- **WHEN** status runs with `TALA_AGENT` or `TALA_AGENT_FILE` and no explicit selector
- **THEN** its JSON data identifies the environment selector source and reports the same public identity and project fields when available

#### Scenario: Status reports no or invalid identity
- **WHEN** status runs without a selector or with an invalid/conflicting selector
- **THEN** it reports an explicit `none` or `invalid` selection state while still returning daemon diagnostics
- **AND** it does not expose the raw selector value or selector-file path

### Requirement: Status output is consistent across formats
Status SHALL report selector, identity, project, and board high-water context consistently in JSON and human-readable output.

#### Scenario: Human status shows context and availability
- **WHEN** status runs with `--human`
- **THEN** it labels the selector source, identity ID/name, registered project root, and board latest ID when available
- **AND** it clearly explains when identity context or the board high-water mark is unavailable

#### Scenario: Board high-water mark is read-only
- **WHEN** status reports `board_latest_id`
- **THEN** the value is the latest message ID in the registered project's board at inspection time
- **AND** reading it does not acknowledge a board message or persist a per-agent cursor
