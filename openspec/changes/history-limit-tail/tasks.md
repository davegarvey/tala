# history --limit newest-N — tasks

## 1. Store: newest-N limit in get_messages_filtered (src/store.rs)

- [ ] 1.1 Replace `result.into_iter().take(limit)` with
      `skip(result.len().saturating_sub(limit))` so the tail (newest N) is kept
      in ascending order
- [ ] 1.2 Verify `limit == 0` is already filtered by callers (`filter(|&l| l > 0)`)
      — keep the `None`/`Some(0)` contract unchanged

## 2. e2e tests (tests/e2e.rs) — written FIRST

- [ ] 2.1 `test_recap_limit_tail`: send m1..m4, `history --json --limit 2`
      returns m3,m4 (assert ids/content), not m1,m2
- [ ] 2.2 `test_wait_limit_tail`: send m1..m4, `wait --since 0 --limit 2 --json`
      returns m3,m4 (newest of matching set)
- [ ] 2.3 Extend `test_recap_limit_cap` to assert newest ids (m2,m3 for 3 msgs)
- [ ] 2.4 `--limit 0` unlimited test unchanged (regression)

## 3. Verify

- [ ] 3.1 `cargo fmt --check` clean
- [ ] 3.2 `cargo clippy -- -D warnings` clean
- [ ] 3.3 `cargo test` green (full suite)
- [ ] 3.4 Live check vs shared daemon: 4-msg session, `history --limit 2`
      prints ids 3,4 with `cursor: 4`
