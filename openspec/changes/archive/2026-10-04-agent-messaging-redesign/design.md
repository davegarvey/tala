## Context

The current CLI uses CWD-local identity, session selection, and cursors; the daemon persists overlapping full-store JSON snapshots. See proposal.md for motivation. The user explicitly authorizes breaking compatibility.

## Goals / Non-Goals

Goals: independently addressed local agent instances, complete CLI access, durable receipts, small implementation modules.
Non-goals: remote hosts, automatic agent wake-up, task scheduling, repository write arbitration, treating peer messages as user authorization.

## Decisions

- Register a UUID agent instance with a name, tool, canonical checkout path, branch and revision. Return an agent ID; require TALA_AGENT or --agent for identity-bearing operations. Never persist an active agent in project files. Names can repeat; resolve exact IDs or unambiguous names, reporting candidate IDs on ambiguity.
- Store agent credentials in the user-private Tala home, not the checkout. Every agent operation carries its credential. This protects against accidental sender substitution; filesystem access to the same user's home is not a security boundary.
- Keep localhost HTTP daemon transport. Use a single versioned RPC envelope and protocol gate, no CORS. Acquire a lifetime file lock before opening SQLite or publishing daemon metadata. A startup lock serializes clients. Stop through the daemon endpoint, never a stale PID. Persist diagnostics to daemon.log.
- Use SQLite with WAL, foreign keys, synchronous FULL and a busy timeout. Commit every message and receipt before acknowledging. Global integer message IDs; UUID thread IDs. Idempotency compares the complete canonical send operation, including routing and intent, within the sender namespace.
- Direct requests have one recipient. Replies are permitted only from that recipient and route to the original sender. An explicit reply settles a request; follow-up requests do not. FYI is default, --request or --wait expects a reply. Handoff creates a request with summary and existing thread context. Pending exposes incoming/outgoing obligations, including received requests, with explicit cancellation via resolve.
- Inbox selection and acknowledgments occur in one transaction. --peek does not acknowledge; history/search/board never acknowledge. Bounded ascending pages expose next_after and has_more. Reads acknowledge only selected items, never a high-water mark over skipped messages.
- Blocking operations poll durable state with a monotonic deadline and brief pauses; no transient event dependence. Polling supports restart recovery and cannot lose messages to broadcast lag. --timeout=0 is indefinite. Presence shows last_seen, registered/inactive and observed listening leases, not an invented availability claim.
- Message parts are text, file references and JSON data. Context records checkout, branch and revision; file references do not transfer files. Inputs are mutually exclusive. JSON emits one consistent envelope per command; timeout exit 3 includes stored request when applicable, errors exit 1, usage exit 2.
- Integration generation is explicit through init, with check/refresh/dry-run modes, no project identity or gitignore writes. Install generic instructions under .tala/AGENTS.md and the OpenCode pair where present, covering Claude/Codex registration and peer-message authority.

## Risks / Trade-offs

- Polling adds up to 200ms delivery latency → favors durable simplicity over duplicated streaming logic.
- Registration survives process exit without reliable tool hooks → report inactivity after five minutes and allow explicit unregister; offline messages remain retrievable on reuse of the ID.
- Old identities cannot be mapped correctly → preserve legacy JSON and report it in status; fresh database only. Roll back with the old binary and untouched old files, after stopping the new daemon.
- Broad rewrite → replace obsolete tests with contract tests plus concurrency, crash, isolation, and CLI integration tests.

## Migration Plan

Stop any old daemon with its matching binary. Build the new binary, use isolated TALA_HOME for validation, register each agent independently, refresh integrations explicitly. Bump wire protocol and release a breaking minor version while pre-1.0. No silent imports or compatibility aliases. Sync delta removals, remove empty superseded main-spec directories, install new capability specs, validate all, then archive on this branch.
