# Spec Delta

## MODIFIED Requirements

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

#### Scenario: Context is available independently of daemon state
- **WHEN** status has a valid selector and the local database is readable while the daemon is stopped, stale, or incompatible
- **THEN** it reports the persisted identity, registered project root, and board latest ID from the local snapshot
- **AND** the daemon's stopped/stale/incompatible state remains separately visible in daemon diagnostics
