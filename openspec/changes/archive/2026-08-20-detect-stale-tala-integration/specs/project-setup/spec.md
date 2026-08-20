## MODIFIED Requirements

### Requirement: Opencode skill installation

`tala init` SHALL install the tala opencode skill and command files into the project's `.opencode/` directory when one exists, so the project's agent knows how to use tala. Each installed Tala integration document SHALL include the current CLI generation version and the minimum Tala CLI version required by the documented commands. The integration documents SHALL preserve a separate skill-content version for tracking instruction changes. `tala init` SHALL not overwrite existing integration documents unless the user explicitly requests a refresh. `tala init --check` SHALL inspect integration state without writing, and `tala init --refresh` SHALL explicitly replace the integration pair. Both forms SHALL accept `--json`. A successful check SHALL return zero after reporting any inspectable status; only an inability to inspect the selected project SHALL be an error. A successful refresh SHALL return zero, while a render or replacement failure SHALL return nonzero. Integration checks SHALL classify the document pair as `absent`, `current`, `stale`, `incompatible`, or `unknown`; a partial pair or invalid/inconsistent metadata SHALL be `unknown`. The project root SHALL be the nearest ancestor of the current directory containing `.tala/config.json` or `.opencode/`, falling back to the current directory when no such ancestor exists.

#### Scenario: Init installs skill and command

- **WHEN** user runs `tala init` and a `.opencode/` directory exists in the project without one or both Tala integration files
- **THEN** the missing skill file SHALL be created at `.opencode/skills/tala/SKILL.md`
- **AND** the missing command file SHALL be created at `.opencode/commands/tala.md`
- **AND** both files SHALL identify the CLI version that generated them
- **AND** both files SHALL identify the minimum CLI version required by their documented commands
- **AND** the skill file SHALL identify its skill-content version separately

#### Scenario: Init preserves existing integration documents

- **WHEN** user runs `tala init` and the project already contains Tala integration documents
- **THEN** `tala init` SHALL leave those documents unchanged
- **AND** `tala init` SHALL tell the user how to explicitly refresh them

#### Scenario: Explicit refresh updates integration documents

- **WHEN** user runs the documented explicit refresh form of `tala init` and a `.opencode/` directory exists
- **THEN** the Tala skill and command documents SHALL be rendered from the current CLI's embedded templates
- **AND** the documents SHALL contain current compatibility metadata
- **AND** `./.tala/config.json` SHALL remain unchanged
- **AND** the CLI SHALL report which integration documents were refreshed

#### Scenario: Init checks integration without modifying files

- **WHEN** user runs the documented check form of `tala init`
- **THEN** the CLI SHALL report the project's integration status as `absent`, `current`, `stale`, `incompatible`, or `unknown`
- **AND** the report SHALL identify the project root and the installed CLI version
- **AND** the check SHALL not modify `.tala/config.json` or either integration document

#### Scenario: Check output is machine-readable

- **WHEN** user runs the documented check form of `tala init` with `--json`
- **THEN** stdout SHALL contain one JSON object with the project root, status, installed CLI version, and integration file paths
- **AND** the command SHALL not write human-readable diagnostics to stdout

#### Scenario: Refresh output is machine-readable

- **WHEN** user runs `tala init --refresh --json` successfully
- **THEN** stdout SHALL contain one JSON object with the project root, current status, and refreshed integration file paths
- **AND** the command SHALL not write human-readable diagnostics to stdout

#### Scenario: Check accepts no identity name

- **WHEN** user combines the documented check form of `tala init` with a positional project identity name
- **THEN** the CLI SHALL reject the arguments as conflicting
- **AND** the command SHALL not modify `.tala/config.json` or integration documents

#### Scenario: Refresh accepts no identity name

- **WHEN** user combines the documented refresh form of `tala init` with a positional project identity name
- **THEN** the CLI SHALL reject the arguments as conflicting
- **AND** the command SHALL not modify `.tala/config.json` or integration documents

#### Scenario: Refresh replaces the integration pair atomically

- **WHEN** explicit refresh fails while rendering or replacing either integration document
- **THEN** the CLI SHALL leave both pre-existing integration documents unchanged
- **AND** the command SHALL return a nonzero error

#### Scenario: Refresh replaces customized documents only explicitly

- **WHEN** existing integration documents contain local edits and the user runs explicit refresh
- **THEN** the CLI MAY replace those documents because refresh is an explicit overwrite operation
- **AND** the CLI SHALL report the paths it replaced

#### Scenario: Integration check from a nested directory uses one project root

- **WHEN** user runs an integration check or refresh from a directory below the nearest ancestor containing the project Tala markers
- **THEN** the CLI SHALL inspect or update the integration files under that same nearest project root
- **AND** it SHALL report the selected project root

#### Scenario: Init with no .opencode directory

- **WHEN** user runs `tala init` and no `.opencode/` directory exists
- **THEN** no skill or command files SHALL be installed
- **THEN** init SHALL still succeed with its config output
