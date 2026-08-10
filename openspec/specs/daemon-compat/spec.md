## Purpose

CLI/daemon version compatibility in tala: the daemon advertises a protocol version and the CLI verifies it before issuing commands, so a stale daemon can never silently break a newer CLI.

## ADDED Requirements

### Requirement: Protocol version advertisement

Every daemon SHALL publish a protocol version. The version SHALL be present in `daemon.json`, in the daemon's live status response (`/api/status`), and in `tala status` output (human and `--json`). The version SHALL be an integer incremented only when the wire protocol changes in an incompatible way. Compatibility checks SHALL compare against the daemon's live status version, not the on-disk `daemon.json` alone (disk and live can disagree if the daemon restarted externally).

#### Scenario: Status shows protocol version
- **WHEN** a user runs `tala status`
- **THEN** the output SHALL include the daemon's protocol version in both human and `--json` forms

#### Scenario: daemon.json carries the version
- **WHEN** a daemon writes `daemon.json` on startup
- **THEN** the file SHALL include the daemon's protocol version

### Requirement: Compatibility check before use

The CLI SHALL verify that the running daemon's protocol version is compatible before issuing any command. When the CLI spawns a daemon itself, the spawned daemon's version SHALL be compatible by construction and the check SHALL pass. When the CLI connects to a running daemon whose version is incompatible, the CLI SHALL fail before issuing the command, printing an error that names both the daemon's version and the CLI's required version and suggests the remedy (restart or upgrade); the command SHALL exit with a nonzero status, and in `--json` mode the error SHALL be emitted as the standard JSON error document. An incompatible daemon SHALL NOT be issued any command that mutates state.

Read-only inspection commands (`tala status`, `tala discover`, `tala agents`) SHALL be exempt from the hard failure: they SHALL report the mismatch as a warning and continue, so users can inspect a stale daemon.

#### Scenario: Fresh spawn is compatible
- **WHEN** a user runs `tala send` and the CLI spawns a new daemon
- **THEN** the send proceeds without a version error

#### Scenario: Stale daemon blocks commands
- **WHEN** a running daemon reports protocol version 1 and the CLI requires version 2
- **THEN** `tala send` SHALL fail with an error naming both versions
- **AND** no message SHALL be stored

#### Scenario: Read-only commands warn instead of failing
- **WHEN** a running daemon's protocol version is incompatible with the CLI
- **THEN** `tala status`, `tala discover`, and `tala agents` SHALL print a warning about the mismatch
- **AND** each SHALL exit with status 0
