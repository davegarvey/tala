# Connection feedback for `listen` and `stream` — tasks

## 1. `cmd_listen` banner + closed note (src/cli.rs)

- [ ] 1.1 After the `resp.status().is_success()` check in `cmd_listen`, print a
      banner: text mode → stdout `Listening on tala daemon <host>:<port> (all
      sessions, since id <N>)...`; `--json` mode → stderr
      `[listen] connected to tala daemon <host>:<port> (since id <N>)`
- [ ] 1.2 Count `message` events received during the stream (increment in the
      existing event loop, both text and JSON paths)
- [ ] 1.3 After the `while let Some(chunk)` loop ends, print the closed note:
      text → stdout `[connection closed] (<count> message(s))`; `--json` →
      stderr `[listen] connection closed (<count> message(s))`
- [ ] 1.4 Cursor write logic (`max_msg_id > since_id`) unchanged

## 2. `cmd_watch` (stream) banner (src/cli.rs)

- [ ] 2.1 After the `resp.status().is_success()` check in `cmd_watch`, print a
      banner: text → stdout `Streaming session <sid> from tala daemon
      <host>:<port> (since id <N>)...`; `--json` → stderr `[stream] connected to
      tala daemon <host>:<port> (session <sid>, since id <N>)`
- [ ] 2.2 Existing end-state output (`[no messages received]`, `[session
      closed]`, JSON closed event) unchanged

## 3. Tests (tests/e2e.rs — write first)

- [ ] 3.1 `test_listen_banner_text`: spawn `listen --since 0 --timeout 3` text
      mode, stderr piped; assert exit 0, stderr contains the connected banner
      and the closed note, stdout is empty (no traffic)
- [ ] 3.2 `test_listen_banner_json_pure_stdout`: spawn `listen --since 0 --json
      --timeout 3`, stderr piped; send a message during the window; assert
      stdout lines parse as JSON (message event) and contain NO banner text;
      stderr contains `connected` and `connection closed`
- [ ] 3.3 `test_listen_closed_note_count`: text mode with a message sent during
      the window; assert the closed note carries the message count
- [ ] 3.4 `test_stream_banner`: spawn `stream <sess> --timeout 3` text mode;
      assert stderr/stdout shows the stream banner; existing stream tests
      unchanged and green
