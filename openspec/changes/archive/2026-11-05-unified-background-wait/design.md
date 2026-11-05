# Design

`tala wait` is a CLI-side poller over the existing inbox and board operations. It queries inbox with `peek=true` and the listening lease, and queries board pages using an independent cursor. No wire operation or protocol version change is required.

Board responses gain an additive `latest_id` field. The store reads the project-scoped maximum message ID in the same transaction as the page. A wait without an explicit `--board-after` uses that high-water mark from one startup read; a missing field is treated as an actionable compatibility error instead of using a partial page cursor.

The wait combines inbox and board events by global message ID. A board reply delivered to the inbox appears once with both sources. The result contains only IDs, event types, and sources; message bodies remain available through the exact inbox or board fetch commands. The `listen_again` command carries the new independent cursors, timeout, and canonical explicit project path.

Inbox and board pages are bounded to 50. Each cursor advances to that source's last returned row, and `has_more` indicates whether another page remains. An earlier cursor can replay events. The command does not persist or acknowledge a notification cursor, avoiding loss when a process exits before the caller receives its result.

A brand-new wait establishes its board baseline at startup. If it stops before returning a cursor, a later brand-new wait establishes a fresh baseline; intervening board history remains retrievable with `tala board`.
