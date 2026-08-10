## MODIFIED Requirements

### Requirement: Pending view

`tala pending` SHALL list all open obligations across the user's sessions: messages with `intent: "req"` that have no correlated reply, plus messages with `expect_reply: true`. Closed sessions SHALL be excluded; reopening a closed session SHALL re-derive its obligations. A `req` answered by an uncorrelated `reply`, or closed by the sender's `out`, SHALL NOT be listed.

Each entry SHALL show the session, sender, message id, a snippet of the message's first text part, and how long the request has been unanswered. A message with no text part SHALL be shown with a placeholder derived from its first non-text part.

#### Scenario: Pending shows unanswered requests
- **WHEN** session `sess_ab12` contains a `req` from `alpha` that no message references via `reply_to`
- **THEN** `tala pending` SHALL list that request with its elapsed time
- **AND** once `beta` sends a `reply` with `reply_to` referencing it, a subsequent `tala pending` SHALL NOT list it

#### Scenario: Pending excludes closed sessions
- **WHEN** an unanswered `req` exists in a session
- **AND** the session is closed
- **THEN** `tala pending` SHALL NOT list it
- **AND** if the session is reopened, `tala pending` SHALL list it again

#### Scenario: Pending snippet uses first text part
- **WHEN** a `req` message contains a text part "please check parse_row" followed by a file part
- **THEN** the pending entry SHALL show the snippet "please check parse_row"

#### Scenario: No text part yields placeholder
- **WHEN** a `req` message contains only a file part
- **THEN** the pending entry SHALL show a `[file]` placeholder instead of an empty snippet

#### Scenario: Pending is empty
- **WHEN** every `req` in every open session has a correlated reply or was closed by its sender's `out`, and no message has `expect_reply: true`
- **THEN** `tala pending` SHALL exit successfully with no entries listed
