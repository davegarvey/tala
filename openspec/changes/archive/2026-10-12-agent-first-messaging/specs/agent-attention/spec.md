# Spec Delta

## Purpose

Tells an agent, in one bounded call, which messages and requests need its action and how to act on each, without burying current work under stale history.

## ADDED Requirements

### Requirement: Attention summary
Running `tala` with no subcommand and a selected identity SHALL return, without acknowledging any message: incoming unanswered requests oldest first, the count and first page of unread messages, and the agent's own outstanding requests. Each item SHALL include an exact `next` command. The summary SHALL be bounded by the standard list limit.

#### Scenario: Request waiting
- **WHEN** an agent with one unanswered incoming request and two unread messages runs `tala`
- **THEN** the result lists the request with a `next` reply command, reports two unread messages, and a following `tala inbox --peek` still shows them unread

#### Scenario: Nothing to do
- **WHEN** an agent with no unread messages and no open requests runs `tala`
- **THEN** the result has empty lists and exit code 0

#### Scenario: No identity selected
- **WHEN** `tala` runs without `--agent` or `TALA_AGENT`
- **THEN** it fails with `IDENTITY_REQUIRED` and a `next` command that registers an identity

### Requirement: Ambient attention
Every successful response to a command run with a selected identity SHALL include an `attention` field, computed in the same transaction as the command, holding the unread count, up to three oldest non-stale incoming unanswered request IDs with sender and `age_seconds`, the stale request count and a `next` command. The field SHALL be omitted when all counts are zero, SHALL contain no message bodies and SHALL acknowledge nothing.

#### Scenario: Learning of a request while posting
- **WHEN** an agent with one unanswered incoming request runs `tala post`
- **THEN** the response includes the posted message and an `attention` field naming the request, and the request remains unread

#### Scenario: Nothing waiting
- **WHEN** an agent with no unread messages and no open requests runs any command
- **THEN** the response has no `attention` field

#### Scenario: Reflects the command's own effect
- **WHEN** an agent consumes its last unread message with `tala inbox`
- **THEN** the same response's `attention` field reports no unread messages

### Requirement: Stale requests
A request SHALL be stale when it is older than 24 hours or its counterpart agent is unregistered. The attention summary and `pending` SHALL report stale requests as a count with the command to list them, and `pending --stale` SHALL list them. Staleness SHALL not change request state.

#### Scenario: Old request collapsed
- **WHEN** an agent has one fresh and twelve 30-day-old unanswered incoming requests
- **THEN** the summary lists the fresh request and reports twelve stale requests with `tala pending --stale` as the next command

#### Scenario: Stale request still answerable
- **WHEN** an agent replies to a stale request
- **THEN** the request becomes answered as normal

### Requirement: Machine-readable age
Messages and requests in JSON output SHALL include `age_seconds` computed by the daemon at response time alongside the stored creation timestamp. The `--human` renderer SHALL show ages in days, hours and minutes.

#### Scenario: Age reported
- **WHEN** a request created 90 minutes ago is listed with JSON output
- **THEN** its `age_seconds` is approximately 5400
