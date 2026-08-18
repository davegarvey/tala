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
skill-content version for tracking instruction changes.

#### Scenario: Init installs skill and command

- **WHEN** user runs `tala init` and a `.opencode/` directory exists in the project
- **THEN** a skill file SHALL be created at `.opencode/skills/tala/SKILL.md` instructing the agent on using tala commands
- **AND** a command file SHALL be created at `.opencode/commands/tala.md`
- **AND** both files SHALL identify the CLI version that generated them
- **AND** both files SHALL identify the minimum CLI version required by their documented commands
- **AND** the skill file SHALL identify its skill-content version separately

#### Scenario: Init with no .opencode directory

- **WHEN** user runs `tala init` and no `.opencode/` directory exists
- **THEN** no skill or command files SHALL be installed
- **THEN** init SHALL still succeed with its config output
