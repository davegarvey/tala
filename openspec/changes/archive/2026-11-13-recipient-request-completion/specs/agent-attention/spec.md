## MODIFIED Requirements

### Requirement: Pending request states
Pending output SHALL distinguish active requests awaiting acceptance from accepted requests awaiting reply, and SHALL identify overdue acceptance deadlines without presenting expiry as execution failure. Declined, answered, cancelled, and superseded requests SHALL not count as actionable pending work or attention. Inactive sender requests SHALL remain open and stale according to existing rules; inactivity alone SHALL NOT expire them.

#### Scenario: Pending shows commitment status
- **WHEN** a sender lists outgoing requests
- **THEN** each active request identifies whether it is awaiting acceptance or accepted and awaiting reply, with any deadline status

#### Scenario: Deadline is overdue but work is not failed
- **WHEN** an acceptance deadline has expired
- **THEN** pending shows the overdue acceptance while retaining the request as open

#### Scenario: Declined request leaves actionable attention
- **WHEN** a recipient declines a request
- **THEN** it no longer appears in incoming or outgoing pending requests, attention request lists, or stale-request counts
- **AND** its recorded decision remains available in history

#### Scenario: Inactive sender request remains open
- **WHEN** a sender becomes inactive while its request is unanswered
- **THEN** the request remains open and is marked stale rather than automatically expired
