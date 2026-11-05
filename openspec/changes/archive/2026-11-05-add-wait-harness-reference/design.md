# Design

## Context

`wait` reports activity without returning message bodies or acknowledging delivery. Inbox `--peek` reads without acknowledging; a normal inbox read acknowledges returned messages. Both inbox and board use a global message-ID cursor. Board reads never acknowledge inbox deliveries. `wait` timeout exits with code 3. Code 4 is for an operation that stored a message and then failed while waiting for its result; a standalone wait loop does not store messages.

## Goals / Non-Goals

**Goals:**

- Give harness authors a practical loop they can adapt while preserving delivery across supervisor restarts.
- Make cursor advancement follow successful processing and acknowledgement.
- State the single-consumer assumption required for a bounded inbox acknowledgement to match the peeked batch.

**Non-Goals:**

- Add a new CLI operation or change inbox/board delivery semantics.
- Provide a universal message handler; applications supply their own idempotent handler.

## Decisions

- Use `tala wait` as a notification, then explicitly fetch inbox messages with `--peek` and board messages with `--after`.
- Persist each fetched payload before invoking the configured handler. The handler must deduplicate by message ID because a board reply may also be delivered to the inbox and a crash before acknowledgement can replay a message.
- Acknowledge inbox work with `--after=<saved-cursor> --limit=<peeked-count>`. The limit avoids sweeping in messages arriving while the handler runs. This corresponds to the peeked batch only when this process is the identity's sole inbox reader; the CLI has no ack-by-ID operation.
- Save inbox cursor from the acknowledgement response only after processing succeeds. Save board cursor from the board response only after processing succeeds. Write cursors through a temporary file in the same directory followed by rename.
- Initialize cursors from known state. A zero board cursor intentionally causes a historical board catch-up; document this as safe but potentially noisy.
- For the wait command, exit 0 means fetch, exit 3 means re-arm with unchanged cursors, and other failures mean back off and inspect status. Do not assign exit 4 to `wait`; it cannot arise from this read-only command path.

## Risks / Trade-offs

- A second inbox reader can acknowledge a peeked message before this loop; then the bounded read selects the earliest remaining unread messages. Mitigate by running one reader per identity and deduplicating all processing by message ID.
- A crash after handling but before acknowledgement replays the batch. The application handler must be idempotent.
- A crash after acknowledgement but before cursor persistence may cause a later unread page to be fetched from an older cursor; acknowledged messages are no longer unread, so they are not delivered again by the inbox.
- A zero initial board cursor can replay existing posts. Let operators supply a deliberate starting cursor when catch-up is not desired.
