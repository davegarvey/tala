## MODIFIED Requirements

### Requirement: Opencode skill installation

`tala init` SHALL install the tala opencode skill and command files into the
project's `.opencode/` directory when one exists, so the project's agent knows
how to use tala. On repeat runs, initialization SHALL create missing
integration files, leave identical files unchanged, and SHALL NOT overwrite a
different existing integration file unless the user passes `--force`. A
different existing file SHALL produce a warning that identifies the path and
the explicit overwrite option.

#### Scenario: Init installs skill and command

- **WHEN** user runs `tala init` and a `.opencode/` directory exists in the
  project
- **THEN** a skill file SHALL be created at `.opencode/skills/tala/SKILL.md`
  instructing the agent on using tala commands
- **AND** a command file SHALL be created at `.opencode/commands/tala.md`

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
- **AND** the existing `.tala/config.json` SHALL remain untouched

## ADDED Requirements

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
- **AND** the report SHALL include an action for `.tala/config.json`
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
