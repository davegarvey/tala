# Design

## Context

Direct replies already set the request's `answered_by` message ID, and the open-request index drives pending and thread attention. The current sender-side `resolve` command cancels a request; recipient completion needs a separate command. Decline removes the open-request index, but `pending --direction=all` currently adds declined outgoing requests back to the list.

## Goals / Non-Goals

**Goals:** link an existing real answer to a request, preserve its provenance, validate recipient authority and answer routing, and clear actionable counts for declined requests.

**Non-Goals:** automatic expiry of inactive senders; moving or rewriting answers; accepting request completions without an existing message.

## Decisions

- Add `complete` as a top-level CLI verb and a `CompleteRequest` operation with request ID and answer message ID.
- In one transaction, require the actor to be the addressed recipient and the request to be open, then verify the answer has a greater global message ID and is an ordinary direct message from that actor to the original sender. It may be a regular message or reply; thread and `reply_to` need not match because users may have answered in another thread.
- Store the answer in the existing `answered_by` field and write a durable `request_completed` event for the sender with the answer ID in event detail. This leaves the answer's original thread intact.
- Permit one ordinary answer to be linked to multiple requests from the same sender, which supports a consolidated response; each request has its own completion event and link.
- Delete the request from `thread_open_requests` when linking it. A repeat with the same answer is idempotent; a different answer after completion fails.
- Remove the declined-request exception from `pending --direction=all`; decisions remain visible on the request record and event history.
- The new wire operation is not understood by older daemons, so integration must bump protocol 11 to 12.

## Risks / Trade-offs

- A completion event is visible to the sender asynchronously; the request record remains the canonical link to the answer. The transaction updates both consistently.
- Answer IDs are global message IDs, so validation must check both directions and direct-message type before writing the cross-link.
