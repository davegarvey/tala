# Unified background wait

## Why

Agents that need direct messages, replies to their requests, and project-board updates currently need separate wait commands. The existing inbox wait also acknowledges messages before a background agent has acted on the notification.

## What changes

- Add `tala wait`, which watches unread inbox items and the selected project's board, with an indefinite default and optional timeout.
- Return event identifiers and types without message bodies. Keep inbox receipts unchanged and provide separate inbox and board cursors plus exact fetch and restart commands.
- Add an atomic project-board high-water mark so a new wait can skip existing board history without paging through a changing board.

## Scope

This change does not alter `inbox --wait` or `send --wait` foreground semantics, add daemon push delivery, or store a notification cursor. A caller that loses its result can replay an earlier cursor; duplicates are safe because inbox items remain unread and board messages remain available in history.
