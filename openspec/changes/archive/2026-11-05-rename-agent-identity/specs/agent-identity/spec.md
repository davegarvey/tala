## ADDED Requirements

### Requirement: Rename the selected identity
The system SHALL allow an agent to rename its authenticated identity in place. A rename SHALL preserve the agent ID, private credential, project context, inbox, messages, and thread memberships. Names SHALL remain non-unique, and recipient resolution SHALL retain its existing exact-ID and ambiguous-name behavior. Renaming to the current name SHALL succeed without changing identity data. Rename SHALL require the current identity credential and SHALL NOT recover or issue a lost handle.

#### Scenario: Rename preserves identity and history
- **WHEN** an agent renames itself using its selected private handle
- **THEN** its ID, credential, project context, messages and thread memberships remain unchanged and its new name appears in identity and agent listings

#### Scenario: Duplicate name remains ambiguous
- **WHEN** an agent renames itself to a name already used by another agent
- **THEN** both identities remain separate, name addressing reports the existing ambiguity error, and exact public IDs still address each identity

#### Scenario: Same name is a no-op
- **WHEN** an agent renames itself to its current name
- **THEN** the operation succeeds with `changed:false` and leaves the identity unchanged

#### Scenario: Rename requires the selected credential
- **WHEN** a rename is sent without a valid selected identity credential or with a blank name
- **THEN** it fails without changing any agent name

#### Scenario: Identical retry returns the recorded result
- **WHEN** the daemon receives the same rename request ID and payload again
- **THEN** it returns the original result without repeating a different mutation
- **AND** reusing that request ID for a different payload fails
