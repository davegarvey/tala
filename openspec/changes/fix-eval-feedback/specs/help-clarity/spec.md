## ADDED Requirements

### Requirement: chat command help surfaces send alias
The `tala chat` (send) command help text SHALL indicate that `tala send` is an alias.

#### Scenario: help shows alias
- **WHEN** user runs `tala chat --help`
- **THEN** the help output mentions that `tala send` is an alias

### Requirement: other hidden aliases remain unchanged
Existing hidden aliases (`follow`→`stream`, `watch`→`stream`, `observe`→`listen`) SHALL remain unchanged.

#### Scenario: hidden aliases still work
- **WHEN** user runs `tala follow` or `tala observe`
- **THEN** the commands work as before with deprecation warnings
