# Session-name resolution (B035) — tasks

## 1. Shared resolver (src/cli.rs)

- [x] 1.1 Add `resolve_session_ref(host, port, input, cmd_name)`:
      exact-name-on-open → unique id; else id exact/prefix → unique id;
      ambiguous → error listing ids; none → `session '<input>' not found`.
      Reuse the fetch-sessions pattern from `cmd_use`.
- [x] 1.2 `cmd_use` keeps its behavior; resolver shares the same matching
      semantics (name-first, then id/prefix).

## 2. Wire into commands (src/cli.rs)

- [x] 2.1 `Send` dispatch: positional session arg is passed to resolver even
      without `sess_` prefix (message disambiguation preserved: single
      positional = message; positional + message = session ref).
- [x] 2.2 `cmd_send`: resolve explicit ref (positional + `--session`) through
      the helper; on no-match, error — never fall back to active session.
- [x] 2.3 `resolve_session_id`: when given an explicit arg, resolve through the
      helper instead of returning it raw (fixes `history`/`close`/`stream`).
- [x] 2.4 `cmd_session_rename`, `cmd_session_reopen`, `cmd_wait`: resolve their
      session arg through the helper.

## 3. Tests (tests/e2e.rs, written first)

- [x] 3.1 send by name routes to the named session (active set to a DIFFERENT
      session; assert message lands in named, not active)
- [x] 3.2 send --session by name works
- [x] 3.3 send positional name with no-match → error, nothing sent anywhere
- [x] 3.4 history by name works
- [x] 3.5 close by name works
- [x] 3.6 rename by name works
- [x] 3.7 ambiguous name (two sessions same name) → error listing ids
- [x] 3.8 regression: lone positional message still sends to active

## 4. Docs (repo)

- [x] 4.1 `.opencode/commands/tala.md`: note session args accept id-or-name
      for send/history/close/rename/wait
