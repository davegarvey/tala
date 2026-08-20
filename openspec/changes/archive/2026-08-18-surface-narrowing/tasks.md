# Tasks — surface narrowing (cycle-19)

## 1. Remove commands

- [x] 1.1 Remove `Stream` variant, dispatch arm, `cmd_stream`, and precheck
      (daemon-compat allow-mismatch) entries; update help cross-references in
      wait/listen
- [x] 1.2 Remove `Agents` variant, dispatch, `cmd_agents`, precheck entries
- [x] 1.3 Remove `SessionCommands::{List, Close, Show}` + dispatch; keep
      create/rename/reopen
- [x] 1.4 Remove/update e2e tests covering removed commands (stream, agents,
      session list/close/show incl. helpers)

## 2. Add send --name

- [x] 2.1 e2e (red): `send --name X "msg"` with no session creates named
      session, sends, sets active; `send -s <id> --name X` errors
- [x] 2.2 clap arg + dispatch; cmd_send auto-create path passes the name;
      explicit-session + --name → usage error

## 3. wait --timeout 0 = indefinite

- [x] 3.1 e2e (red): `wait -s <id> --timeout 0` stays parked and delivers a
      message sent after start
- [x] 3.2 cli: pass 0 through for wait and wait-new (no default substitution);
      text shows "(no timeout)"
- [x] 3.3 daemon: session-wait and wait-new-stream treat timeout_secs 0 as no
      deadline

## 4. Ambiguity guard

- [x] 4.1 e2e (red): two open sessions + bare send → stderr warning names
      active + count; stdout/JSON unchanged
- [x] 4.2 cli: warn in the active-session fallback path of cmd_send/cmd_wait

## 5. Help/docs

- [x] 5.1 `--new-session` flag help describes real semantics (B049)
- [x] 5.2 `--expect-reply` on req/out error suggests `--intent req`/`--wait`
      (B051)
- [x] 5.3 SKILL.md: auto-create correction; drop stream/agents/session
      list/show/close; document send --name + wait --timeout 0
- [x] 5.4 README: command table + golden path example updated

## 6. Validation

- [x] 6.1 fmt + clippy clean; full e2e green
- [x] 6.2 Live: golden path end-to-end on fresh daemon (send --name → wait
      --new-session → reply → pending → close); removed commands error;
      timeout-0 parked wait delivers; guard warns
