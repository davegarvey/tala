# Spec Delta

## MODIFIED Requirements

### Requirement: Stop and startup are linearizable
Daemon startup SHALL reserve the current lifecycle generation before attempting startup coordination and SHALL publish an instance only while that generation remains current. An accepted stop SHALL advance the generation, prevent earlier reservations from publishing afterward, target the instance active at stop acceptance, and report success only after that instance exits. An instance SHALL close its storage before it releases its lifetime lock, so release of that lock means the instance no longer holds the database open. No lifecycle or startup lock SHALL be held while waiting for shutdown.

#### Scenario: Stop races with an unpublished start
- **WHEN** a start reservation exists but has not published its daemon instance when stop is accepted
- **THEN** that reservation cannot publish after the stop generation advances
- **AND** stop-in-progress remains set even when no daemon target has published
- **AND** stop waits for the startup barrier before reporting success

#### Scenario: Stop targets the active instance
- **WHEN** stop is accepted while an instance is active and a later explicit command starts a replacement
- **THEN** stop waits for the originally targeted instance to exit
- **AND** the replacement is not mistaken for the target or killed by that stop

#### Scenario: Explicit command starts after stop
- **WHEN** an ordinary explicit command needs a daemon while an instance is stopping
- **THEN** it waits without holding lifecycle locks and may start a replacement after the target exits

#### Scenario: Stop marker survives an in-flight stop
- **WHEN** the target removes daemon metadata before releasing its lifetime lock
- **THEN** stop-in-progress remains set until that lock is available
- **AND** a new wait exits terminally while that state remains set

#### Scenario: Stop releases storage
- **WHEN** stop reports success for a targeted instance
- **THEN** that instance has closed its database before releasing its lifetime lock
- **AND** the database file can be moved or deleted on every supported platform
