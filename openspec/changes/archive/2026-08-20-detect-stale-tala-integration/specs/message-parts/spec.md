## MODIFIED Requirements

### Requirement: Parts rendering

Every message surface (`history`, `wait`, `listen`, `check`) SHALL render text parts as the message content, exactly as a plain string message today. File parts SHALL render as an annotation naming the path (e.g. `[file: src/api.rs]`), and data parts as an annotation with the JSON value (e.g. `[data: {"status":"ok"}]`), shown after the text parts in part order. In `--json` output, the full typed `parts` array SHALL be present on every message. Message match filters (e.g. `listen --match`) SHALL match against the message's text parts only; a message with no text part SHALL NOT match.

#### Scenario: History shows file annotation

- **WHEN** a message contains a text part "fix applied" and a file part `src/api.rs`
- **THEN** `tala history` SHALL render the text "fix applied" and the annotation `[file: src/api.rs]`

#### Scenario: JSON output exposes typed parts

- **WHEN** a user runs `tala history --json` on a session containing a message with text and data parts
- **THEN** the message JSON SHALL include a `parts` array whose elements carry their kind and payload
