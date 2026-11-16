# Spec Delta

## Purpose

Lets a coding agent learn of Tala work while it is busy on a task, and keep its identity across context compaction, through opt-in connectors that adapt one harness-neutral hook runner to each harness's own hook or plugin mechanism.

## ADDED Requirements

### Requirement: Connector catalogue
The system SHALL bundle a catalogue of connectors, each identified by a name and described by a manifest declaring the harness events it handles, the files or configuration entries it installs at each supported scope, and how it renders hook output. `tala connector list` SHALL report each connector's name, harness, supported scopes and install state. The catalogue SHALL include `claude-code`, `codex` and `opencode`.

#### Scenario: List connectors
- **WHEN** a user runs `tala connector list`
- **THEN** the result lists `claude-code`, `codex` and `opencode` with their supported scopes and whether each is installed, stale or absent at each scope

#### Scenario: Unknown connector
- **WHEN** a user runs `tala connector install unknown`
- **THEN** Tala fails with a usage error naming the available connectors and exits with code 2

### Requirement: Harness-neutral core
Harness names, harness configuration formats and harness payload fields SHALL appear only in connector manifests and templates. Messaging, attention, identity and wait commands SHALL behave identically whether or not any connector is installed.

#### Scenario: Core unaffected by connectors
- **WHEN** a connector is installed and an agent runs `tala`, `tala inbox` or `tala wait`
- **THEN** output and receipt behaviour match an installation without connectors

### Requirement: Explicit installation
`tala connector install <name>` SHALL change harness configuration only for the named connector and the selected `--scope=user|project` (default `user`). It SHALL merge entries into existing configuration without altering unrelated content, SHALL be idempotent, SHALL support `--dry-run`, and SHALL report every path it creates or changes. Installed hook commands SHALL invoke Tala by the absolute path of the installing binary.

#### Scenario: Install preserves existing configuration
- **WHEN** a user installs a connector into a harness configuration file that already contains other hooks and settings
- **THEN** those hooks and settings are unchanged and the Tala entries are added

#### Scenario: Repeated install
- **WHEN** a user installs the same connector at the same scope twice
- **THEN** the second run reports every path as unchanged and adds no duplicate entries

#### Scenario: Dry run
- **WHEN** a user runs `tala connector install codex --dry-run`
- **THEN** Tala reports the planned changes and writes nothing

#### Scenario: Unreadable configuration
- **WHEN** the target harness configuration exists but cannot be parsed
- **THEN** Tala changes no file, fails with an error naming the path, and exits with code 1

### Requirement: Harness approval is reported
When a harness requires the user to review or trust new hooks before they run, install output SHALL state that step and where to perform it. Tala SHALL NOT mark hooks as trusted or bypass a harness's approval mechanism.

#### Scenario: Codex hooks await review
- **WHEN** a user installs the `codex` connector
- **THEN** the result states that the hooks run only after the user reviews them in Codex (Settings → Hooks in the desktop app, `/hooks` in the CLI)

### Requirement: Removal and checking
`tala connector remove <name>` SHALL remove only the entries and files that connector installed at the selected scope and leave unrelated configuration unchanged. `tala connector check [<name>]` SHALL report each installation as current, stale (installed by another Tala version or pointing at a missing binary) or absent, and SHALL change nothing.

#### Scenario: Remove restores prior configuration
- **WHEN** a user installs and then removes a connector at the same scope
- **THEN** the harness configuration is semantically equal to its state before installation

#### Scenario: Stale binary path
- **WHEN** the binary recorded in an installed hook no longer exists
- **THEN** `tala connector check` reports the installation as stale with `tala connector install <name>` as the next command

### Requirement: Session-scoped identity
The hook runner SHALL derive a session key from the harness payload, and SHALL use a private selector file for that key under the Tala home as the session's identity. Hooks SHALL never register an agent, and context injected by a hook SHALL name the selector file path but never contain a private handle.

#### Scenario: Registered session
- **WHEN** an agent has registered with `--save-handle-to` set to its session's selector path
- **THEN** later hooks in that session report attention for that identity

#### Scenario: Two sessions in one checkout
- **WHEN** two harness sessions in the same directory each register through their own session selector paths
- **THEN** each session's hooks report only its own identity's attention

### Requirement: Session start and compaction reminders
On a session start or resume in a project initialised with `tala init`, the hook runner SHALL inject a short instruction giving the exact registration command with the session's selector path. When the session already has a selector, including after compaction where the harness reports it, the runner SHALL instead inject a reminder to use `--agent-file` with that path. In other projects it SHALL inject nothing.

#### Scenario: New session in an initialised project
- **WHEN** a session starts in a project containing `.tala/AGENTS.md` and has no selector file
- **THEN** the injected context gives a `tala register` command with `--save-handle-to` set to the session's selector path

#### Scenario: After compaction
- **WHEN** a registered session's context is compacted and the harness reports it
- **THEN** the injected context names the selector path to pass with `--agent-file`, without the handle

#### Scenario: Unrelated project
- **WHEN** a session starts in a project without `.tala/AGENTS.md` and has no selector file
- **THEN** the hook produces no output

### Requirement: Change-only attention notifications
After a tool call, the hook runner SHALL report the session identity's attention summary only when it contains an open request or unread message not reported to that session before, or when previously reported items remain unresolved for 15 minutes since they were last reported. Otherwise it SHALL produce no output. Reporting SHALL NOT acknowledge messages or change request state.

#### Scenario: New request arrives mid-task
- **WHEN** a peer sends a request to a registered session's identity and the session then completes a tool call
- **THEN** the hook output names the request with its `next` command and the message remains unread

#### Scenario: Unchanged attention is not repeated
- **WHEN** the same open request is present on three consecutive tool calls within 15 minutes
- **THEN** only the first produces output

#### Scenario: Unresolved request is repeated
- **WHEN** a reported request is still open 15 minutes after it was last reported
- **THEN** the next tool call reports it again

### Requirement: Hooks never disrupt the harness
The hook runner SHALL exit successfully with no output when the session has no selector, the selector is invalid, the daemon is not running or unreachable, or any other error occurs. It SHALL NOT start a daemon, SHALL complete within two seconds, and SHALL NOT block or alter the tool call that triggered it.

#### Scenario: Daemon not running
- **WHEN** a hook runs while no Tala daemon is running
- **THEN** it exits 0 with no output and no daemon is started

#### Scenario: Unregistered session
- **WHEN** a tool-call hook runs in a session with no selector file
- **THEN** it exits 0 with no output
