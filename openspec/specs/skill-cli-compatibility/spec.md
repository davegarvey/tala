## Purpose

Makes Tala agent instructions self-describing and lets agents detect when the
installed CLI is too old or when their project-local skill files are stale.

## Requirements

### Requirement: Skill documents declare CLI compatibility

Every Tala skill document installed for an agent SHALL contain machine-readable
metadata with a distinct skill-content version, a minimum supported Tala CLI
version, and the Tala CLI version that generated the document. The CLI version
fields SHALL use Semantic Versioning 2.0.0 values and SHALL NOT be confused
with the skill-content version. The skill-content version MAY retain the
existing document-version format.

#### Scenario: Skill metadata is present

- **WHEN** an agent reads an installed Tala skill document
- **THEN** it can identify the skill-content version
- **AND** it can identify `tala_cli_min_version`
- **AND** it can identify `tala_cli_generated_version`

#### Scenario: Metadata uses valid versions

- **WHEN** the skill document is validated
- **THEN** both CLI version fields SHALL be parseable as Semantic Versioning
  2.0.0 values
- **AND** the generated CLI version SHALL identify the binary that produced the
  document

#### Scenario: Version precedence is semantic

- **WHEN** an agent compares CLI versions
- **THEN** it SHALL use Semantic Versioning precedence, including prerelease
  ordering
- **AND** build metadata SHALL NOT affect precedence

#### Scenario: Generated metadata is self-consistent

- **WHEN** a skill document is rendered by a Tala binary
- **THEN** `tala_cli_min_version` SHALL be less than or equal to
  `tala_cli_generated_version` under Semantic Versioning precedence
- **AND** the renderer SHALL reject metadata that violates this invariant

### Requirement: Skill guidance checks the installed CLI

The Tala skill SHALL instruct an agent to treat `tala --version` as the
authoritative installed CLI version and to compare it semantically with
`tala_cli_min_version` before using version-specific commands or flags.

#### Scenario: Installed CLI is older than the minimum

- **WHEN** `tala --version` reports a version lower than
  `tala_cli_min_version`
- **THEN** the agent SHALL warn that the CLI is incompatible with the skill
- **AND** the agent SHALL avoid relying on commands or flags requiring the
  newer CLI
- **AND** the agent SHALL recommend upgrading Tala

#### Scenario: Installed CLI is older than the generated version

- **WHEN** the installed CLI satisfies `tala_cli_min_version` but is older than
  `tala_cli_generated_version`
- **THEN** the agent SHALL treat the skill as potentially incompatible
- **AND** the agent SHALL not assume that documented commands or flags exist
- **AND** the agent SHALL recommend upgrading Tala or checking the installed
  binary's help before proceeding

#### Scenario: Installed CLI is newer than the generated version

- **WHEN** the installed CLI is newer than `tala_cli_generated_version`
- **THEN** the agent SHALL treat the skill as stale-document guidance
- **AND** the agent SHALL recommend refreshing the project integration
- **AND** the agent SHALL verify every documented command and flag with the
  installed binary before relying on them, because newer releases may remove
  or rename commands

#### Scenario: Installed CLI satisfies the skill

- **WHEN** the installed CLI satisfies `tala_cli_min_version`
- **AND** the installed CLI version equals `tala_cli_generated_version`
- **THEN** the agent MAY use the commands documented by the skill without an
  incompatibility warning

#### Scenario: Version metadata is missing or invalid

- **WHEN** a Tala skill document lacks a required CLI version field or contains
  an invalid CLI version
- **THEN** the agent SHALL treat CLI compatibility as unknown
- **AND** the agent SHALL warn that the skill needs refreshing
- **AND** the agent SHALL verify the required commands with the installed binary
  before using them

#### Scenario: Version command output is invalid

- **WHEN** `tala --version` fails or does not contain a valid Semantic Versioning
  value after the executable name
- **THEN** the agent SHALL treat CLI compatibility as unknown
- **AND** the agent SHALL not claim that the skill and binary are compatible

### Requirement: Version-aware integrations handle unversioned documents

A version-aware Tala skill integration SHALL treat missing or invalid skill
metadata as unknown compatibility. It SHALL treat valid metadata as guidance
only. The agent SHALL use the locally installed binary's version as the source
of truth and SHALL NOT treat project-controlled skill metadata as proof of
binary identity or security.

#### Scenario: Version-aware agent encounters an unversioned document

- **WHEN** an agent using the version-aware guidance encounters a Tala skill
  document without the required CLI metadata
- **THEN** the agent SHALL warn that the document is unversioned
- **AND** the agent SHALL recommend refreshing the project integration
- **AND** the agent SHALL verify required commands with the installed binary
  before using them

#### Scenario: Project skill metadata is modified

- **WHEN** a project changes the CLI version metadata in its local skill file
- **THEN** the agent SHALL still query `tala --version` before making a
  compatibility decision

### Requirement: CLI detects stale project integration

When a Tala command is invoked from a project containing a repository-local Tala integration, the CLI SHALL compare the integration's CLI metadata with the installed binary. The installed binary SHALL remain authoritative, and a mismatch or unknown metadata SHALL be reported as a warning rather than silently treated as compatible. The skill and command documents SHALL be evaluated as one integration pair.

#### Scenario: Current integration matches the installed CLI

- **WHEN** both nearest project-local Tala integration documents contain valid metadata whose generated CLI version matches the installed CLI
- **THEN** the CLI SHALL not print a compatibility warning

#### Scenario: Integration was generated by an older CLI

- **WHEN** the nearest project-local Tala integration contains valid metadata generated by an older CLI version
- **THEN** the CLI SHALL print a warning naming both CLI versions
- **AND** the warning SHALL state that the documented command surface may be stale
- **AND** the warning SHALL recommend checking `tala --help` and refreshing the integration

#### Scenario: Integration was generated by a newer CLI

- **WHEN** the nearest project-local Tala integration contains valid metadata generated by a newer CLI version
- **THEN** the CLI SHALL print a warning naming both CLI versions
- **AND** the warning SHALL state that the installed CLI may not support all documented commands
- **AND** the warning SHALL recommend upgrading Tala or checking `tala --help`

#### Scenario: Installed CLI is below the documented minimum

- **WHEN** the installed CLI version is lower than the minimum version declared by either integration document
- **THEN** the CLI SHALL print a warning naming the installed and minimum versions
- **AND** the warning SHALL recommend upgrading Tala
- **AND** the warning SHALL not claim that the integration and installed CLI are compatible

#### Scenario: Integration metadata is missing or invalid

- **WHEN** a repository-local Tala integration exists without valid CLI compatibility metadata
- **THEN** the CLI SHALL print a warning that integration compatibility is unknown
- **AND** the warning SHALL recommend refreshing the integration and verifying commands with `tala --help`

#### Scenario: Integration has only one document

- **WHEN** exactly one of the repository-local Tala skill or command documents exists
- **THEN** the CLI SHALL classify the integration as unknown
- **AND** the CLI SHALL print one warning recommending the explicit integration refresh flow

#### Scenario: Integration documents disagree

- **WHEN** both repository-local integration documents exist but their required metadata is invalid or inconsistent
- **THEN** the CLI SHALL classify the integration as unknown
- **AND** the CLI SHALL print one warning recommending the explicit integration refresh flow

#### Scenario: Project has no Tala integration

- **WHEN** a Tala command is invoked from a project without repository-local Tala integration files
- **THEN** the CLI SHALL not print a stale-integration warning

### Requirement: Compatibility warnings do not corrupt command output

Compatibility diagnostics SHALL be non-blocking for otherwise valid commands and SHALL be written to stderr, so normal stdout data and `--json` output remain machine-readable. The CLI SHALL aggregate all integration problems into at most one concise warning per invocation.

#### Scenario: Human-readable command with stale integration

- **WHEN** a valid Tala command is invoked with stale project integration files
- **THEN** the command SHALL continue according to its normal behavior
- **AND** the compatibility warning SHALL be written to stderr

#### Scenario: JSON command with stale integration

- **WHEN** a valid `--json` Tala command is invoked with stale project integration files
- **THEN** the command SHALL continue according to its normal behavior
- **AND** stdout SHALL contain only the command's documented JSON output
- **AND** the compatibility warning SHALL be written to stderr

### Requirement: Unknown-command failures identify possible stale instructions

When an unrecognized Tala subcommand is rejected and an existing repository-local integration is unversioned, incompatible, or stale, the CLI SHALL include an actionable hint that the project instructions may not match the installed command surface. A project with no Tala integration SHALL not receive this hint solely because a subcommand is unknown.

#### Scenario: Removed command appears in stale project instructions

- **WHEN** the user invokes an unrecognized subcommand documented by a stale repository-local Tala integration
- **THEN** the CLI SHALL retain its nonzero usage-error result
- **AND** the error SHALL identify the unrecognized subcommand
- **AND** the error SHALL recommend `tala --help` and refreshing the project integration
