## MODIFIED Requirements

### Requirement: Opencode skill installation

`tala init` SHALL install the tala opencode skill and command files into the
project's `.opencode/` directory when one exists, so the project's agent knows
how to use tala. Each installed Tala integration document SHALL include the
current CLI generation version and the minimum Tala CLI version required by
the documented commands. The integration documents SHALL preserve a separate
skill-content version for tracking instruction changes.

#### Scenario: Init installs skill and command

- **WHEN** user runs `tala init` and a `.opencode/` directory exists in the
  project
- **THEN** a skill file SHALL be created at
  `.opencode/skills/tala/SKILL.md` instructing the agent on using tala
- **AND** a command file SHALL be created at `.opencode/commands/tala.md`
- **AND** both files SHALL identify the CLI version that generated them
- **AND** both files SHALL identify the minimum CLI version required by their
  documented commands
- **AND** the skill file SHALL identify its skill-content version separately

#### Scenario: Init with no .opencode directory

- **WHEN** user runs `tala init` and no `.opencode/` directory exists in the
  project
- **THEN** no skill or command files SHALL be installed
- **AND** init SHALL still succeed with its config output
