## Context

The intent protocol stores a request's `waiting_until` deadline independently
from the derived answer state. The current human renderers therefore continue
to display a live-looking deadline after a later message has answered the
request. Blocking sends also render received messages as sender plus content,
while pending guidance knows the message id but omits the session target.

## Goals / Non-Goals

**Goals:**

- Make every human message surface distinguish an unanswered deadline from a
  settled request.
- Give a blocking sender enough information to correlate the message that
  ended its wait without requiring an immediate follow-up command.
- Make the pending answer hint safe in a multi-session project.
- Preserve the existing wire model, JSON shapes, command names, and stdout
  machine-readability guarantees.

**Non-Goals:**

- Do not clear or rewrite persisted `waiting_until` timestamps when a reply
  arrives; they remain useful historical timing data.
- Do not add a completion command, auto-close sessions, or introduce new flags.
- Do not change pending obligation derivation or intent precedence.

## Decisions

### Derive settled presentation from existing correlation state

The CLI will determine whether a request is answered using the same correlation
rules that drive `pending`: explicit `reply_to`, applicable uncorrelated replies,
and the sender's `out` closure. Renderers will suppress the active deadline or
label the request settled once that state is known. This is preferred over
mutating `waiting_until`, which would lose the original wait deadline and could
make historical timing less useful.

The answer-state calculation should be shared or factored at the existing
message-rendering boundary so `history`, `check`, `wait`, `listen`, and
`pending` do not develop different interpretations. An unanswered expired
request keeps the current `wait expired` presentation and remains pending.

### Reuse the existing intent badge for blocking-send receipts

The human `send --wait` receipt will use the same message id and intent/reply
badge already used by transcript and wait output, rather than inventing a new
receipt format. JSON remains unchanged except where the current response already
exposes the received message object, so scripts do not need to parse human text.

### Generate a fully qualified pending reply hint

The human pending renderer will include `--session <id>` before
`--reply-to <id>`. The session id comes from the obligation already returned by
the daemon. This is preferred over changing active-session behavior: explicit
targeting prevents accidental cross-session replies without adding a command or
changing per-directory state.

## Risks / Trade-offs

- [Different surfaces currently render messages through separate paths] → Add
  focused tests for each affected human surface and centralize only the answer
  state needed for deadline rendering.
- [Human output changes may affect snapshot-like consumers] → Keep changes
  additive where possible, preserve JSON output, and document the new id/badge
  fields as the stable human convention.
- [A reply may be uncorrelated or an `out` message] → Reuse the pending
  derivation rules rather than checking only `reply_to`.
- [Explicit session ids are less concise] → Prefer correctness in a suggested
  command; users can still omit the target when they intentionally rely on an
  active session.

## Migration Plan

1. Implement shared settled-request rendering and update the human outputs.
2. Add/adjust unit and end-to-end coverage for correlation, wait rendering,
   blocking-send receipts, and pending hints.
3. Run the current two-agent eval with both one-session and multi-session cases.
4. Rollback requires only reverting presentation changes; no persisted data or
   protocol migration is needed.
