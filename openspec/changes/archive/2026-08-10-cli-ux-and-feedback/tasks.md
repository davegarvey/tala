## 1. Wait --new-session Initial Feedback

- [x] 1.1 In `cmd_wait_new()`, add an eprintln before the HTTP call: `"Waiting for a new session (timeout: {}s)..."` (only in non-JSON mode)

## 2. Listen --new-only Flag (Deferred — requires server-side changes)

The `--new-only` flag is deferred because the SSE observe endpoint uses `since` for BOTH history replay and new message filtering. A client-only flag cannot reliably skip history without also suppressing new messages. Instead, the help text now documents `--since` as the mechanism.

- [x] 2.1 Update `Listen` after_help to document `--since <n>` for skipping history replay

## 3. Use Without Args Lists Sessions

- [x] 3.1 In `cmd_use()`, in the "no active session" branch, fetch sessions from daemon and list them

## 4. CLI Help Clarity

- [x] 4.1 Update `Wait` doc comment to mention `--new-session` for discoverability
- [x] 4.2 Update `Wait` after_help to reference `stream` and `listen` with brief usage guidance
- [x] 4.3 Update `Stream` after_help with cross-references
- [x] 4.4 Update `Listen` after_help with cross-references

## 5. Cursor Update on Send

- [x] 5.1 In `cmd_send()`, after successful send, write cursor with the message ID from `SendMessageResponse`
