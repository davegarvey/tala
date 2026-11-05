# Spec Delta

## ADDED Requirements

### Requirement: List recent threads from history
The CLI SHALL allow `tala history` without a thread ID to list recent threads, while preserving `tala history <thread>` as the full thread reader. The list SHALL support a bounded `--limit` and a separate `--cursor`; the in-thread `--after` cursor SHALL remain exclusive to full thread reads.

#### Scenario: List recent threads
- **WHEN** an agent runs `tala history` with no thread ID
- **THEN** Tala returns recent threads visible to that selected identity with `has_more`, `next_cursor`, and the snapshot high-water message ID

#### Scenario: Continue thread-list paging
- **WHEN** an agent runs `tala history --cursor=<cursor>`
- **THEN** Tala returns the next page from the same listing snapshot

#### Scenario: Preserve full thread reads
- **WHEN** an agent runs `tala history <thread> --after=<message-id>`
- **THEN** Tala returns the existing paginated full thread history behavior

#### Scenario: Keep the cursor modes separate
- **WHEN** a history invocation combines a thread ID with `--cursor` or omits the thread ID while using `--after`
- **THEN** the CLI returns a structured usage error without making an RPC

### Requirement: Render thread summaries in both formats
Thread-list JSON SHALL return a `threads` array with thread ID, conversational participants, separately identified handoff-only recipients, last message ID and time, message count, unread count, fresh and stale incoming/outgoing pending counts, thread kind and relevant project, and a bounded excerpt. Human output SHALL render the same information as a compact escaped table.

#### Scenario: JSON thread-list fields
- **WHEN** an agent requests a thread list in JSON mode
- **THEN** each row includes the documented summary fields and the response includes paging metadata

#### Scenario: Human thread-list parity
- **WHEN** an agent runs `tala history --human` without a thread ID
- **THEN** Tala prints the same thread rows and summary values in a compact escaped table
