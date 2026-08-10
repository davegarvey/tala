## ADDED Requirements

### Requirement: stdin via --file -
`tala send --file -` SHALL read message content from piped stdin, consistent with the documented convention. It SHALL NOT attempt to open a file named `-` on disk.

#### Scenario: Send message from piped stdin via --file -
- **WHEN** user runs `echo "hello" | tala send --file -`
- **THEN** the message "hello" is sent to the current session

#### Scenario: Send message from piped stdin via --file - with explicit session
- **WHEN** user runs `cat message.txt | tala send sess_abc123 --file -`
- **THEN** the contents of message.txt are sent to session sess_abc123

#### Scenario: --file - with no piped input
- **WHEN** user runs `tala send --file -` without piping input
- **THEN** an error message is displayed indicating no piped input

### Requirement: Self-message exclusion from unread counters
Messages sent by the current agent SHALL NOT increment the unread/new-message counters in `tala list` and `tala status` output.

#### Scenario: Own message not counted as unread
- **WHEN** agent A sends a message in a session
- **THEN** `tala list` SHALL NOT show that message as increasing the unread count for agent A
- **AND** `tala status` SHALL NOT report that message as "new"

#### Scenario: Other agent's message counted as unread
- **WHEN** agent A sends a message in a session
- **AND** agent B sends a message later
- **THEN** `tala list` SHALL show agent B's message as increasing the unread count for agent A
- **AND** `tala status` SHALL report agent B's message as "new"

### Requirement: Consistent cursor/since terminology
`--cursor` SHALL be accepted as an alias for `--since` on `tala recap`. The API response SHALL use `cursor` in JSON output. Internal terminology SHALL be consistent.

#### Scenario: --cursor accepted by recap
- **WHEN** user runs `tala recap --cursor 5`
- **THEN** it behaves identically to `tala recap --since 5`

#### Scenario: --since accepted by recap (unchanged)
- **WHEN** user runs `tala recap --since 5`
- **THEN** it behaves identically to existing behavior
