# Session-name resolution for send/history/close/rename/wait (B035)

## Why

Backlog B035 (new in cycle-13, P1, reconfirmed on main caf8718): session
references by NAME only work in `tala use`. Everywhere else they are rejected
or, worse, silently misrouted:

```
# active = sess_1dfig (other-sess)
$ tala send nr-x "misroute-check"        # ✓ Sent message 1 to session sess_1dfig  (!!)
$ tala send --session nr-x "msg"         # Error: session 'nr-x' not found
$ tala history nr-x                      # Error: session 'nr-x' not found
$ tala close nr-x                        # Error: session 'nr-x' not found
$ tala session rename nr-x new           # Error: session 'nr-x' not found
```

Root cause in `src/cli.rs`: the `Send` dispatch only treats a positional as a
session reference when it starts with `sess_`
(`session.filter(|s| s.starts_with("sess_"))`). A positional NAME is dropped
from session resolution entirely, so `cmd_send` falls through to the active
session (or auto-creates one) — exit 0, no warning. `--session <name>` and
`history`/`close`/`rename` pass the raw string to the daemon, which only
accepts IDs (`store.get_session` is an ID-map lookup).

For an agent messaging tool this is the worst failure mode: a sender believes
they addressed a named counterpart but the message lands in a different
conversation, silently. `use` already implements name→id resolution
(`cmd_use`, cli.rs:595-655: exact name match on open sessions, then id
exact/prefix match, error on ambiguity); the fix is to share that resolution
across every session-taking command.

## What Changes

- **`src/cli.rs`**: extract a shared helper
  `resolve_session_ref(host, port, input, cmd_name) -> anyhow::Result<String>`
  that resolves an input to a session ID:
  1. exact name match among open sessions → id (unique); >1 → error listing ids;
  2. id exact match → id; id-prefix match (unique) → id; ambiguous prefix → error;
  3. no match → error `session '<input>' not found` (never falls back silently).
- Wire the helper into:
  - `cmd_send` when an explicit session ref is given (positional AND `--session`):
    resolve first, then send; on no-match, error out (no active-session fallback
    when the user explicitly named a target).
  - `resolve_session_id` (used by `history`, `close`, `stream`) — resolve the
    given arg through the helper instead of returning it raw.
  - `cmd_session_rename`, `cmd_session_reopen`, `cmd_wait` — resolve their
    session arg the same way (consistency; `use` unchanged, it already works).
- Positional handling in the `Send` dispatch: when a positional session arg is
  present, pass it to the resolver even if it does not start with `sess_`
  (message-vs-session disambiguation stays: a lone positional with no message
  is still a message; a positional followed by a message is a session ref).
- Error text: keep `use`'s phrasing family — `Multiple sessions named '<name>':
  <ids>` and `session '<input>' not found`.

## Capabilities

- `tala send <session-name> "question"` delivers to the session named
  `<session-name>`, never to a random active session, exit 0.
- `tala send --session <name>`, `tala history <name>`, `tala close <name>`,
  `tala session rename <name> <new>`, `tala session reopen <name>`,
  `tala wait <name>` all resolve names.
- Explicit no-match/ambiguous refs fail loudly with the offending input.

## Impact

- CLI behavior change (bug fix): previously-silent misrouting becomes either
  correct routing or a hard error. Existing id-based usage is unchanged
  (ids still resolve). No daemon/API changes; no storage changes.
- Tests: new e2e tests for send-by-name routing, send --session by name,
  history/close/rename by name, no-match error, ambiguous-name error.
- Docs: no README changes needed (README documents `use <name>`; command
  reference in `.opencode/commands/tala.md` gets a note that session args
  accept id-or-name across send/history/close/rename/wait).
