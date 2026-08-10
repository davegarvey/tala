## Purpose

Message input sources for `tala send`: the explicit `--stdin` flag reads the whole message from the piped stream with no timeout and no shell interpretation, while the implicit pipe fallback keeps working without the flag — so messages with backticks, quotes, and leading dashes reach agents intact.

## Requirements

### Requirement: `tala send --stdin` reads the message from stdin

The `--stdin` flag SHALL read the message content from stdin until EOF instead of from a positional argument, bypassing shell interpretation of the message text. When `--stdin` is set, any positional message SHALL be ignored with a warning. `--stdin` with no piped content (stdin is a terminal) SHALL error; piped-but-empty content SHALL also error. `--message-file` SHALL take precedence over `--stdin` when both are given. `--stdin` SHALL block until EOF (no 500 ms timeout). Sending with `--stdin` SHALL behave like any other send: `--session` targets a specific session and `--wait` blocks for a reply.

#### Scenario: Send with --stdin and piped content
- **WHEN** user pipes content into `tala send --stdin`
- **THEN** the piped content SHALL be sent as the message body

#### Scenario: Send with --stdin and no pipe errors
- **WHEN** user runs `tala send --stdin` interactively (stdin is a terminal)
- **THEN** the command SHALL error with "No message provided via stdin"

#### Scenario: Send with --stdin and empty piped content errors
- **WHEN** user runs `printf "" | tala send --stdin`
- **THEN** the command SHALL error (the piped content is empty)

#### Scenario: Positional argument ignored with warning
- **WHEN** user runs `tala send "hello" --stdin`
- **THEN** a warning SHALL be printed: "Warning: --stdin is set, ignoring positional message argument"
- **AND** the message SHALL be read from stdin

#### Scenario: --message-file wins over --stdin
- **WHEN** user runs `tala send --message-file /path/to/file --stdin`
- **THEN** the content SHALL be read from the file and `--stdin` SHALL be ignored

#### Scenario: Send with --stdin and --wait
- **WHEN** user pipes content into `tala send --stdin --wait`
- **THEN** the piped content SHALL be sent and the command SHALL wait for a reply

#### Scenario: Send with --stdin and --session
- **WHEN** user pipes content into `tala send --stdin --session sess_abc`
- **THEN** the piped content SHALL be sent to `sess_abc`

### Requirement: Implicit piped stdin fallback

When content is piped to `tala send` without `--stdin` or `--message-file`, the implicit stdin detection (stdin is not a terminal) SHALL continue to work as a fallback with the existing 500 ms read timeout — preserving the behavior that `tala send` on a terminal never hangs waiting for input.

#### Scenario: Implicit pipe without --stdin
- **WHEN** user runs `echo "hello" | tala send`
- **THEN** "hello" SHALL be sent as the message body (existing behavior preserved)
