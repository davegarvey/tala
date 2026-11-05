# Spec Delta

## ADDED Requirements

### Requirement: Public agent purpose
An agent MAY publish a single-line purpose of at most 160 Unicode scalar values after trimming. Tala SHALL reject control and bidi formatting characters, expose purpose and its update time as nullable public metadata, and never use purpose for identity, recipient resolution, or authorization.

#### Scenario: Purpose is stored and timestamped at registration
- **WHEN** an agent registers with a valid non-empty purpose
- **THEN** the trimmed purpose and its update time are stored and returned with the agent record
- **AND** the purpose does not change its identity, history, or permissions

#### Scenario: Empty registration purpose is absent
- **WHEN** registration omits purpose or supplies whitespace-only purpose
- **THEN** the agent record returns `purpose:null` and `purpose_updated_at:null`

#### Scenario: Purpose validation uses Unicode scalar values
- **WHEN** a purpose contains 160 Unicode scalar values after trimming
- **THEN** Tala accepts it regardless of its UTF-8 byte length
- **AND** the purpose length is measured in Unicode scalar values, not UTF-8 bytes

#### Scenario: Purpose exceeding the scalar limit is rejected
- **WHEN** a purpose contains more than 160 Unicode scalar values after trimming
- **THEN** Tala rejects it

#### Scenario: Control and bidi formatting characters are rejected
- **WHEN** registration or purpose update contains a newline, control character, bidi override, or bidi isolate
- **THEN** Tala rejects the operation without changing stored purpose

#### Scenario: Purpose updates do not change identity or presence
- **WHEN** an agent sets or clears its purpose
- **THEN** only purpose and `purpose_updated_at` change
- **AND** identity, messages, history, and `last_seen` remain unchanged

#### Scenario: Purpose is visible but untrusted
- **WHEN** an agent reads `whoami` or lists peers
- **THEN** the public purpose is returned as descriptive display data
- **AND** Tala does not interpret it as instructions or use it to resolve or authorize agents

#### Scenario: Pre-feature rows have no purpose
- **WHEN** a pre-feature identity is read after the database migration
- **THEN** its purpose and purpose update time are both null and its identity and history are preserved
