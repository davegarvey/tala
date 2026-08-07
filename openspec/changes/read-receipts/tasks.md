# Sender read receipts (B021) — tasks

## 1. Store: daemon-side read state (src/store.rs)

- [x] 1.1 Add `read_state: Arc<RwLock<HashMap<(String, String), u64>>>` field
      (session_id, sender) → last read msg id; init in `Store::new`
- [x] 1.2 `record_read(&self, session_id, sender, up_to)` — insert max
- [x] 1.3 `list_sessions`: attach `read_by: HashMap<String, u64>` (readers of
      this session) to each `SessionSummary`

## 2. Models (src/models.rs)

- [x] 2.1 `SessionSummary` gains `read_by: HashMap<String, u64>` with
      `#[serde(default)]` + `skip_serializing_if = "HashMap::is_empty"`
- [x] 2.2 `RecapQuery` gains `sender: Option<String>`

## 3. API (src/api.rs)

- [x] 3.1 `GetMessagesParams` and `WaitParams` gain `sender: Option<String>`
- [x] 3.2 `recap_session`: after messages computed, if sender present and
      messages non-empty → `record_read(id, sender, max_id)`
- [x] 3.3 `get_messages`: same recording
- [x] 3.4 `wait_for_message`: record on BOTH the existing-messages path and the
      live-delivery path (only when messages actually returned)
- [x] 3.5 `wait_new_session` / `wait_all`: record read for the delivered session
      (they already take a sender param)

## 4. CLI (src/cli.rs)

- [x] 4.1 `cmd_recap`: append `&sender=<identity>` (read_project_config →
      default sender, same as wait)
- [x] 4.2 `cmd_whatsup` (check): append `&sender=<identity>` to each
      `/messages` call
- [x] 4.3 `cmd_list` text: append `read: <agent>@<id>` (comma-separated) for
      readers != local identity
- [x] 4.4 `cmd_list --json`: `read_by` flows through SessionSummary (no change
      needed beyond model field)

## 5. e2e tests (tests/e2e.rs) — written FIRST

- [x] 5.1 `test_read_receipts_after_history`: dirs a (alpha) + b (beta) with
      config.json identities; a sends msg 1; b `history`s the session; a
      `list --json` shows `read_by: {"beta": 1}`; a `list` text contains
      `read: beta@1`
- [x] 5.2 `test_read_receipts_after_wait`: b `wait --timeout` receives msg →
      read_by updated
- [x] 5.3 `test_read_receipts_not_recorded_on_send`: a sends msg 2 → read_by
      unchanged (no alpha entry, beta still @1)
- [x] 5.4 `test_read_receipts_self_read_json_but_not_text`: a `history`s own
      session → json read_by includes alpha@2, text shows only `read: beta@1`

## 6. Verify

- [x] 6.1 `cargo fmt --check` clean
- [x] 6.2 `cargo clippy -- -D warnings` clean
- [x] 6.3 `cargo test` green (full suite)
- [x] 6.4 Live check vs shared daemon: alpha sends, beta reads via history,
      alpha `list` shows `read: beta@1`, `list --json` shows read_by
