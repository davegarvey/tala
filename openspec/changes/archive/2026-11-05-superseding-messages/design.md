# Design

## Context

Direct sends are transactional SQLite rows. `reply_to` records reply correlation and must keep that meaning. A message's serialized `Operation::Send` is also the canonical value compared for explicit-key retries, while the CLI derives automatic keys from a fixed set of send fields. Inbox, history, pending, and attention read through shared message and open-request queries.

## Goals / Non-Goals

**Goals:**
- Store an auditable, durable one-way edge from a replacement message to the earlier direct message it replaces.
- Make thread inference, recipient validation, reply handling, and pending state consistent in the same storage transaction.
- Keep old messages, answers, and receipt timestamps unchanged.
- Refuse v5 daemon connections for the v6 wire operation rather than risk silent loss of amendment metadata.

**Non-Goals:**
- Superseding board posts, board replies, or handoffs.
- Automatically selecting or reusing an identity.
- Editing or deleting historical messages.

## Decisions

1. **Persist one nullable `supersedes` pointer on the successor.** Add a self-referencing `messages.supersedes` column and unique index in schema version 3. Compute `superseded_by` as the inverse relation when loading messages. A unique index permits many nulls while preventing a second successor, and the pointer naturally supports A <- B <- C chains. A separate amendments table or duplicated `superseded_by` column would create more state to keep consistent.

2. **Validate and insert inside the existing send transaction.** Resolve the `--to` value first, load the target, and require a prior message from the same sender to the same resolved recipient in the same direct thread. `--thread` may be omitted, in which case the target thread is inherited; an explicit different thread is rejected. Only `kind=message` successors and direct `message`/`reply` targets are allowed; board messages and `handoff` rows are excluded. SQLite's unique index is the final guard against concurrent branches.

3. **Treat supersession as a separate terminal reason for a request.** Do not alter `received_at`, `answered_by`, or `cancelled`. Amendments suppress the older row from the shared active-request predicate, so pending, attention, and stale counts agree. The unread predicate is unchanged; unconsumed rows remain unread and both relation directions remain visible. A reply to a superseded request fails with `REQUEST_SUPERSEDED`, naming its successor and directing the recipient to inspect the thread.

4. **Preserve ordinary retry identity.** Add `supersedes: Option<u64>` at the end of `Operation::Send` and omit it when `None` during serialization. That keeps pre-feature canonical JSON byte-compatible for ordinary sends. Keep the current derived-key material unchanged when `None`; append the target only when present. For amended sends, canonical and derived keys include the target, so changing it is a distinct send and explicit-key reuse conflicts.

5. **Bump the wire protocol to 6.** A v5 daemon may ignore an unknown optional field and store a message without its amendment relation. Protocol mismatch is safer than silently dropping the new behavior. Existing databases are upgraded transactionally from user_version 2 to 3; existing rows receive null `supersedes` and retain their history.

## Risks / Trade-offs

- **An amendment can arrive after its recipient has read or answered the old request.** Preserve the old receipt/answer state and make the relationship visible; a later amendment does not erase prior work or answers.
- **Two concurrent amendments target one message.** The unique index rejects the second transaction; convert the constraint failure into a useful, non-retrying conflict that identifies the existing successor.
- **Old canonical send rows may be compared after upgrade.** Omitting `None` from serialization and preserving the old derived-key tuple avoids changing ordinary retry identity.
- **Protocol 5 clients cannot use a v6 daemon.** The existing protocol mismatch error directs operators to restart with a matching binary; release installation upgrades the CLI and daemon together.

## Migration Plan

On store open, migrate user_version 2 to 3 in one transaction by adding the nullable self-reference column and unique index. Set `PROTOCOL_VERSION` to 6. Existing rows remain valid and read with both amendment fields null. If rollback is needed, stop v6 clients and restore a v5 binary; the added nullable column and index are additive, but the v5 daemon must not be run concurrently with v6 clients.
