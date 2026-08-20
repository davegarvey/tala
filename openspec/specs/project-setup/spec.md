## Purpose

Project setup in tala: `tala init` creates the project's `.tala/` config with a sender identity, and that identity is used on every message sent from the project.

## Requirements

### Requirement: Project initialization

The system SHALL initialize a project for use with tala by creating a `./.tala/` directory and a `./.tala/config.json` holding the project's agent name.

#### Scenario: Init creates .tala directory with config

- **WHEN** user runs `tala init` in a project directory
- **THEN** a `./.tala/` directory SHALL be created
- **THEN** a `./.tala/config.json` SHALL be created with the project's basename as the default name

#### Scenario: Init does not overwrite existing config

- **WHEN** user runs `tala init` and `./.tala/config.json` already exists
- **THEN** the CLI SHALL print a notice that the config already exists and SHALL leave it untouched

#### Scenario: Init with custom name

- **WHEN** user runs `tala init my-agent`
- **THEN** `./.tala/config.json` SHALL use "my-agent" as the default sender name

### Requirement: Project identity in messaging

The system SHALL use the project name from `./.tala/config.json` as the default sender identity for messages sent from that project.

#### Scenario: Send uses project name

- **WHEN** user runs `tala send <session> "message"` from a project with `./.tala/config.json`
- **THEN** the message SHALL be attributed to the project name from config

#### Scenario: Send without .tala/config.json

- **WHEN** user runs `tala send <session> "message"` from a project without `./.tala/config.json`
- **THEN** the message SHALL be attributed to the current directory's basename

### Requirement: Opencode skill installation

`tala init` SHALL install the tala opencode skill and command files into the
project's `.opencode/` directory when one exists, so the project's agent knows
how to use tala. Each installed Tala integration document SHALL include the
current CLI generation version and the minimum Tala CLI version required by
the documented commands. The integration documents SHALL preserve a separate
skill-content version for tracking instruction changes. On repeat runs,
initialization SHALL create missing integration files, leave identical files
unchanged, and SHALL NOT overwrite a different existing integration file unless
the user passes `--force` or explicitly requests `--refresh`. A different
existing file SHALL produce a warning that identifies the path and the explicit
overwrite option. `tala init` SHALL not overwrite existing integration
documents unless the user explicitly requests one of those operations.
`tala init --check` SHALL inspect integration state without writing, and
`tala init --refresh` SHALL explicitly replace the integration pair. Both forms
SHALL accept `--json`. A successful check SHALL return zero after reporting any
inspectable status; only an inability to inspect the selected project SHALL be
an error. A successful refresh SHALL return zero, while a render or replacement
failure SHALL return nonzero. Integration checks SHALL classify the document
pair as `absent`, `current`, `stale`, `incompatible`, or `unknown`; a partial
pair or invalid/inconsistent metadata SHALL be `unknown`. The project root
SHALL be the nearest ancestor of the current directory containing
`.tala/config.json` or `.opencode/`, falling back to the current directory when
no such ancestor exists.

#### Scenario: Init installs skill and command

- **WHEN** user runs `tala init` and a `.opencode/` directory exists in the project without one or both Tala integration files
- **THEN** a skill file SHALL be created at `.opencode/skills/tala/SKILL.md` instructing the agent on using tala commands
- **AND** a command file SHALL be created at `.opencode/commands/tala.md`
- **AND** both files SHALL identify the CLI version that generated them
- **AND** both files SHALL identify the minimum CLI version required by their documented commands
- **AND** the skill file SHALL identify its skill-content version separately

#### Scenario: Init preserves existing integration documents

- **WHEN** user runs `tala init` and the project already contains Tala integration documents
- **THEN** `tala init` SHALL leave those documents unchanged
- **AND** `tala init` SHALL tell the user how to explicitly refresh them

#### Scenario: Explicit refresh updates integration documents

- **WHEN** user runs `tala init --refresh` and a `.opencode/` directory exists
- **THEN** the Tala skill and command documents SHALL be rendered from the current CLI's embedded templates
- **AND** the documents SHALL contain current compatibility metadata
- **AND** `./.tala/config.json` SHALL remain unchanged
- **AND** the CLI SHALL report which integration documents were refreshed

#### Scenario: Init checks integration without modifying files

- **WHEN** user runs `tala init --check`
- **THEN** the CLI SHALL report the project's integration status as `absent`, `current`, `stale`, `incompatible`, or `unknown`
- **AND** the report SHALL identify the project root and the installed CLI version
- **AND** the check SHALL not modify `.tala/config.json` or either integration document

#### Scenario: Check output is machine-readable

- **WHEN** user runs `tala init --check --json`
- **THEN** stdout SHALL contain one JSON object with the project root, status, installed CLI version, and integration file paths
- **AND** the command SHALL not write human-readable diagnostics to stdout

#### Scenario: Refresh output is machine-readable

- **WHEN** user runs `tala init --refresh --json` successfully
- **THEN** stdout SHALL contain one JSON object with the project root, current status, and refreshed integration file paths
- **AND** the command SHALL not write human-readable diagnostics to stdout

#### Scenario: Check accepts no identity name

- **WHEN** user combines `tala init --check` with a positional project identity name
- **THEN** the CLI SHALL reject the arguments as conflicting
- **AND** the command SHALL not modify `.tala/config.json` or integration documents

#### Scenario: Refresh accepts no identity name

- **WHEN** user combines `tala init --refresh` with a positional project identity name
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

#### Scenario: Repeat init leaves identical integration files unchanged

- **GIVEN** the installed integration files already equal the current Tala
  generated content
- **WHEN** user runs `tala init`
- **THEN** the files SHALL not be rewritten
- **AND** init SHALL report that they are unchanged

#### Scenario: Repeat init skips a locally different file

- **GIVEN** an existing integration file differs from the current generated
  content
- **WHEN** user runs `tala init` without `--force`
- **THEN** the existing file SHALL remain byte-for-byte unchanged
- **AND** init SHALL warn that the file was skipped
- **AND** the warning SHALL name `tala init --force` as the explicit overwrite
  action

#### Scenario: Force refresh overwrites integration files

- **GIVEN** an existing integration file differs from the current generated
  content
- **WHEN** user runs `tala init --force`
- **THEN** the file SHALL be replaced with the current generated content
- **AND** the existing `./.tala/config.json` SHALL remain untouched

### Requirement: Init reports and previews actions

`tala init` SHALL report the actions it takes or skips. It SHALL accept
`--dry-run` to calculate and report the same plan without writing project
files. It SHALL accept `--json` / `-j` to emit a machine-readable action report
on stdout; human warnings and diagnostic logs SHALL remain on stderr.

#### Scenario: Dry run makes no changes

- **WHEN** user runs `tala init --dry-run`
- **THEN** init SHALL report whether config, integration files, and requested
  ignore rules would be created, unchanged, skipped, or overwritten
- **AND** init SHALL not create, modify, or delete any project file

#### Scenario: JSON reports actions

- **WHEN** user runs `tala init --json`
- **THEN** stdout SHALL contain valid JSON
- **AND** the report SHALL include an action for `./.tala/config.json`
- **AND** the report SHALL include an action for each detected integration file
- **AND** the report SHALL include an action for Git-ignore setup when
  `--gitignore` is supplied

#### Scenario: Force is explicit in JSON mode

- **WHEN** user runs `tala init --json` without `--force` and an integration file
  differs
- **THEN** the JSON report SHALL identify the file as skipped
- **AND** the JSON report SHALL include a warning that `--force` is required

### Requirement: Git-ignore setup is opt-in

`tala init` SHALL NOT modify a `.gitignore` file unless the user supplies
`--gitignore`. With that option, init SHALL add a single `/.tala/` pattern to
the repository-root `.gitignore` when the project is inside a Git repository and
the pattern is not already present. Running the option repeatedly SHALL NOT
duplicate the pattern.

#### Scenario: Init does not edit Git-ignore rules by default

- **WHEN** user runs `tala init` without `--gitignore`
- **THEN** init SHALL leave all `.gitignore` files unchanged

#### Scenario: Git-ignore option adds the project state pattern

- **WHEN** user runs `tala init --gitignore` inside a Git repository whose root
  `.gitignore` lacks `/.tala/`
- **THEN** init SHALL add `/.tala/` to the repository-root `.gitignore`
- **AND** init SHALL report that the rule was added

#### Scenario: Existing Git-ignore rule is preserved

- **WHEN** user runs `tala init --gitignore` and the repository already ignores
  `.tala/`
- **THEN** init SHALL not duplicate or rewrite the existing rule
- **AND** init SHALL report that the rule was already present

#### Scenario: Git-ignore option outside a repository

- **WHEN** user runs `tala init --gitignore` outside a Git repository
- **THEN** init SHALL not create or modify a `.gitignore` file
- **AND** init SHALL warn that no Git repository root was found
