## ADDED Requirements

### Requirement: Project initialization
The system SHALL initialize a project for use with chit.

#### Scenario: Init creates .chit directory with config
- **WHEN** user runs `chit init` in a project directory
- **THEN** a `./.chit/` directory SHALL be created
- **THEN** a `./.chit/config.json` SHALL be created with the project's basename as the default name

#### Scenario: Init does not overwrite existing config
- **WHEN** user runs `chit init` and `./.chit/config.json` already exists
- **THEN** the CLI SHALL skip creation or prompt before overwriting

#### Scenario: Init with custom name
- **WHEN** user runs `chit init --name "my-project"`
- **THEN** `./.chit/config.json` SHALL use "my-project" as the default sender name

### Requirement: Project identity in messaging
The system SHALL use the project name from `./.chit/config.json` as the default sender identity.

#### Scenario: Send uses project name
- **WHEN** user runs `chit send <session> "message"` from a project with `./.chit/config.json`
- **THEN** the message SHALL be attributed to the project name from config

#### Scenario: Send without .chit/config.json
- **WHEN** user runs `chit send <session> "message"` from a project without `./.chit/config.json`
- **THEN** the message SHALL be attributed to the current directory's basename

### Requirement: Opencode skill generation
The system MAY generate an opencode skill file for chit integration.

#### Scenario: Init generates opencode skill
- **WHEN** user runs `chit init --opencode`
- **THEN** a skill file SHALL be created at `./.chit/opencode-skill.md`
- **THEN** the skill file SHALL instruct the agent on using chit commands and markdown formatting
