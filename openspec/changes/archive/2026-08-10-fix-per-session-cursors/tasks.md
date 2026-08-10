# Fix broken read-model: per-session cursors — tasks

## 1. Cursor store (src/store.rs)

- [x] 1.1 Replace `local_cursor_path` (`.tala/cursor`, single u64) with
      `cursors_path()` → `.tala/cursors.json` storing `HashMap<String, u64>`
- [x] 1.2 `read_cursors() -> HashMap<String, u64>` (missing/corrupt file → empty)
- [x] 1.3 `read_cursor(session_id: &str) -> u64` (default 0)
- [x] 1.4 `write_cursor(session_id: &str, cursor: u64)` — merge + persist
- [x] 1.5 Legacy `.tala/cursor` ignored, not deleted

## 2. CLI read-model (src/cli.rs)

- [x] 2.1 `cmd_send`: `write_cursor(&msg.session_id, msg.id)` (B023)
- [x] 2.2 `cmd_recap` (history): `write_cursor(&session_id, ...)`; empty session
      → no write (B025)
- [x] 2.3 `cmd_list`: read cursors map once; per-session unread (B014)
- [x] 2.4 `cmd_status`: `compute_total_unread` over per-session cursors
- [x] 2.5 `cmd_whatsup` (check): per-session fetch + per-session cursor updates;
      JSON keeps `"cursor"` (max) + adds `"cursors"`; text prints updated-marker
      count
- [x] 2.6 `cmd_listen`: when `--since` absent, pass `since_map` (URL-encoded
      JSON of per-session cursors) to `/api/observe`
- [x] 2.7 `compute_session_unread` / `compute_total_unread` signatures take the
      per-session cursor / cursors map

## 3. Server (src/api.rs)

- [x] 3.1 `observe_events` accepts optional `since_map` (JSON string param);
      per-session replay uses `map.get(&session.id).copied().unwrap_or(since)`

## 4. Tests first (tests/e2e.rs)

- [x] 4.1 B014: session A (3 msgs, read via history) + new session B (1 msg) →
      `list` shows B with `(1 new)`, A without; `check --json` reports the B
      message and `"cursors"` map has per-session entries
- [x] 4.2 B025: read A; `history` on empty session B → A's unread state unchanged
      (no spurious `(N new)`); cursor file still contains A's marker
- [x] 4.3 B023: send to low-id session does not hide another session's unread;
      sender's own session cursor updates but nothing else changes
- [x] 4.4 check per-session: `check` marks only the sessions it saw; a second
      `check` reports nothing new
- [x] 4.5 listen (no `--since`): message in a NEW session is replayed even
      though its id is below another session's cursor
- [x] 4.6 existing cursor-dependent tests (`test_recap_cursor`, whatsup-adjacent)
      still pass

## 5. Verification

- [x] 5.1 `cargo fmt --check`
- [x] 5.2 `cargo clippy --all-targets -- -D warnings`
- [x] 5.3 `cargo test` (CARGO_HOME=/workspace/.cargo-home)
- [x] 5.4 manual: fresh-daemon repro from cycle-05 feedback — beta cursor high,
      alpha new session + msg → `list` shows `(1 new)`, `check` reports it
