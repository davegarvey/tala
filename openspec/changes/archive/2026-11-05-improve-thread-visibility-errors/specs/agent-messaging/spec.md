# Spec Delta

## ADDED Requirements

### Requirement: Non-disclosing thread visibility errors
Operations that reference a thread outside the selected identity's visible scope SHALL return `THREAD_NOT_VISIBLE`. This failure SHALL use the same public message and hint for inaccessible and missing threads, SHALL NOT reveal handles or foreign project metadata, and SHALL set `next` to `tala guide identity`.

#### Scenario: History references an inaccessible thread
- **WHEN** an agent requests history for a missing thread or one outside its visible scope
- **THEN** Tala returns the generic `THREAD_NOT_VISIBLE` failure with identical public fields for both cases

#### Scenario: Handoff references an inaccessible thread
- **WHEN** an agent hands off a thread outside its visible scope
- **THEN** Tala returns the same non-disclosing `THREAD_NOT_VISIBLE` failure as history

#### Scenario: Another thread operation references an inaccessible thread
- **WHEN** an agent tries to continue, inspect, or reply within a thread outside its visible scope
- **THEN** Tala returns the same non-disclosing `THREAD_NOT_VISIBLE` failure
