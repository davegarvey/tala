# wait-new-freshest-session (B029) — tasks

## 1. API: read-state-aware scan (src/api.rs)

- [x] 1.1 Add `seen: Option<String>` (URL-encoded JSON map session_id→last-seen)
      to `WaitNewParams`; parse into `HashMap<String, u64>`
- [x] 1.2 `wait_new_session`: pass parsed `seen` into `find_incoming_session`
- [x] 1.3 `find_incoming_session(store, me, seen)`: candidate = open session
      with NO cursor entry in `seen` AND ≥1 message where `sender != me`;
      rank by freshest incoming message timestamp; return best candidate +
      its freshest qualifying incoming message. Sessions with a cursor entry
      (waiter created/sent-in/read them) are excluded outright
- [x] 1.4 Event loop: unchanged (live NewMessage from another agent satisfies)

## 2. CLI: send read state (src/cli.rs)

- [x] 2.1 `cmd_wait_new`: read `store::read_cursors()`, URL-encode JSON, append
      `&seen=<encoded>` to the wait-new URL (tiny percent-encode helper for the
      JSON chars — no new crate, offline-safe)
- [x] 2.2 `cmd_wait` no-active-session fallback: same `seen` param
- [x] 2.3 `auto_create_session`: `store::write_cursor(&session.id, 0)` so
      waiter-created sessions are "seen" from birth

## 3. Tests (tests/e2e.rs) — write FIRST

- [x] 3.1 Consumed message is NOT re-delivered: alpha sends msg to S, beta
      `history S` (cursor=1), beta `wait --new-session --timeout 3 --json` →
      `{"timeout":true}`; then alpha's new session+msg → wait returns it
- [x] 3.2 Fresh-never-seen beats stale-never-seen: S1 (alpha msg, beta never
      engaged) + S2 (alpha msg, created after S1) both exist before beta waits
      → returns S2, not S1 (freshest wins)
- [x] 3.3 Waiter-created session is excluded: beta creates S1, alpha replies;
      alpha creates S2 + msg; beta `wait --new-session` → S2 (S1 has a cursor
      entry from beta's create, excluded)
- [x] 3.4 B003 regression guard: alpha's pre-existing session + question
      (never-seen by beta) is still returned immediately (existing test must
      keep passing)

## 4. Verify

- [x] 4.1 `cargo fmt --check` clean
- [x] 4.2 `cargo clippy --all-targets -D warnings` clean
- [x] 4.3 Full `cargo test` passes (incl. #46 suite)
- [x] 4.4 Live repro on shared daemon: stale session not delivered; fresh
      handshake delivered; re-wait converges instead of looping on stale
