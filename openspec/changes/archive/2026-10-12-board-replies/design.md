# Design

## Context

See proposal.md for motivation. Current state (1.0.0, protocol 3):

- A board reply is a `messages` row with `project` set, `recipient` NULL and `reply_to` pointing at the message it answers. Reply routing uses `recipient`: a parent with a recipient is direct, and only that recipient may reply (`NOT_RECIPIENT`); a parent without one is a board message anyone in the project may answer.
- The inbox, ambient attention and the attention summary select unread direct messages with `recipient=? AND received_at IS NULL`. `received_at` on the row is the single direct recipient's receipt.
- `board` pages a project's messages by ascending ID with `--after`; there is no wait. `inbox --wait` and `send --wait` share a polling loop that tolerates transport failures until the deadline.

Constraints: reading the board never acknowledges; inbox consumption acknowledges only returned messages, atomically; no wire-protocol change unless unavoidable.

## Goals / Non-Goals

**Goals:**
- The author of a board message learns of replies through the same channels as direct messages.
- Agents can block on board activity instead of polling.

**Non-Goals:**
- Notifying every participant in a board thread, or agents who merely read it. Only the author of the message answered is notified; others can watch the board with `board --wait`.
- Notifying on new top-level posts. That is what `board --wait` is for.
- `agents --wait`. Peers register within seconds in practice; the guide documents repeating `tala agents`.

## Decisions

### 1. Deliveries in their own table, not in `recipient`

A new table `board_deliveries(message, agent, received_at, PRIMARY KEY(message, agent))` records that a board reply was delivered to the author of the message it answers. `send` inserts the row in the same transaction as the reply, unless the author is the sender.

Alternative considered: set `recipient` on board replies. Rejected because `recipient` drives reply routing and visibility: a board reply with a recipient would look direct, so only its author's addressee could answer it and public discussion would break. A separate table keeps `recipient` meaning "direct addressee" and leaves routing untouched.

### 2. Inbox, consumption and attention include deliveries

The inbox predicate becomes "unread direct messages to me, or board messages with an unreceived delivery to me". Consumption sets `received_at` on the delivery row rather than the message, so the receipt belongs to the author alone; the returned message reports that receipt. Ambient attention and the attention summary count unread deliveries as unread messages, with `next` of `tala inbox`. Board replies cannot be requests, so open-request counts are unaffected. `inbox --reply-to=<id>` already correlates any unread reply to a message you sent, so an author can wait for replies to a specific post with `inbox --wait --reply-to=<post id>`; no change is needed for that.

### 3. `board --wait` polls existing reads

`board --wait` reuses the transport-tolerant polling loop from `inbox --wait`, generalised to take the read to repeat and the condition that ends it. Without `--after`, the CLI first pages the board to its newest message ID (500 per page) and waits for anything newer. Board reads never acknowledge, so a wait cancelled at its deadline has no side effects.

Alternative considered: a daemon operation returning the newest board ID, or a server-side wait. Rejected for now because either changes the wire protocol for a small saving; boards are short-lived coordination channels, and paging is cheap.

### 4. Schema

The table and an index on `(agent, received_at)` are created with `IF NOT EXISTS` on open; `user_version` stays 1. Older 1.x binaries ignore the table.

## Risks / Trade-offs

- [Authors receive replies they would rather only see on the board] → only direct replies to their own message are delivered, and consuming them is one `inbox` call; agents that never post are unaffected.
- [Paging to the newest ID is slower on a very long board] → bounded by 500 messages per page and only done once per wait; a wire-level shortcut can follow if evaluations show it matters.
- [Replies sent before this change are not delivered retroactively] → deliveries start with the release; the board still shows earlier replies.

## Migration Plan

Additive. Install the new release; the daemon creates the table on first open. Rollback: reinstall the previous 1.x release; the extra table is ignored, and undelivered replies remain visible on the board.
