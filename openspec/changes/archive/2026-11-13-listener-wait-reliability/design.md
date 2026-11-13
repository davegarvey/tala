# Design

## Context

See proposal.md. The CLI currently gives `timeout=0` a finite transport recovery deadline, while the daemon already treats zero as unbounded. Durable overdue generation already writes a per-message timestamp before recording its event.

## Goals / Non-Goals

**Goals:** Preserve indefinite waits across reconnects; allow inbox-only listeners to ignore board and durable housekeeping; make event resumption explicit; report meaningful recipient state and avoid alerts for inactive recipients.

**Non-Goals:** Change message acknowledgement, alter default wait behavior, migrate stored messages, or break CLI/daemon compatibility.

## Decisions

- Represent `--only=inbox` as a narrow additive wait mode. The daemon advances a private polling cursor across ignored housekeeping events to avoid repeated wakeups, but returns the caller's original event cursor so those events remain available if the caller later resumes an unfiltered wait. Inbox reads remain non-consuming and source cursors remain independent.
- Keep the existing overdue marker as the once-only guard. Check recipient activity when processing the threshold and set the marker even when suppressing the sender event, so an inactive recipient cannot cause repeated scans or an alert after later state changes.
- Add a clear recipient state alongside the existing listening boolean. This distinguishes inactive from active-but-not-listening without changing the established presence signal.
- Make the CLI's transport deadline optional just like the user deadline: zero remains `None`, positive values retain the existing recovery margin.
- Expose the already advanced event cursor as `next_event_after` and use it in continuation output.

## Risks / Trade-offs

- Keeping the caller's event cursor means ignored durable events can be scanned again by a later inbox-only wait. → Keep the daemon's private cursor advancing during the current blocking call, while returning the original cursor in its response.
- Recipient activity can change after send or threshold processing. → Report send presence as a point-in-time observation and evaluate activity again when overdue processing runs.

## Migration Plan

No data migration is required. The new CLI option and response fields are additive; keep the existing protocol version unless integration determines the combined wire changes require a bump.
