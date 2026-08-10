# Tasks — cycle-18 eval feedback fixes

## 1. B039: session create --json success output

- [ ] 1.1 e2e test: `tala session create --name X --json` prints a JSON doc
      containing `session_id`, exits 0 (red before fix)
- [ ] 1.2 `cmd_session_create`: print `{"session_id": "<id>"}` in json mode;
      text mode unchanged
- [ ] 1.3 e2e: `send` auto-create with `--json` still prints exactly one JSON
      document (no double output)

## 2. B040: reply-intent correlation warning

- [ ] 2.1 e2e test: `send --intent reply "msg"` (no --reply-to) prints a
      warning on stderr, still sends, stdout unchanged
- [ ] 2.2 `send_message`: warn on stderr when resolved intent == reply and
      reply_to is None

## 3. B041: wait-new-session name

- [ ] 3.1 e2e test: `wait --new-session --json` result includes `name` for a
      named session
- [ ] 3.2 api.rs: add `name` to the four wait-new-stream result events
- [ ] 3.3 cli.rs: text mode prints bare id to stdout (capture contract),
      name to stderr; json mode passes name through

## 4. Validation

- [ ] 4.1 `cargo fmt --check`, clippy `-D warnings`, full `cargo test` green
- [ ] 4.2 Live check on a fresh daemon: all three behaviors verified
