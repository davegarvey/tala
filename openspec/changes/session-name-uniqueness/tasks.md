# Session name uniqueness (B017) — tasks

## 1. Store (src/store.rs)

- [ ] 1.1 Add `pub async fn session_name_exists(&self, name: &str) -> bool` —
      any session in `self.sessions` with `name == Some(name)`.
- [ ] 1.2 `rename_session`: before mutating, scan sessions; if a session with
      `id != session_id` already has `name == Some(target)` → return
      `Err(format!("A session named '{}' already exists", target))`.
      Self-rename (same name) stays Ok(true). Existing `_force` semantics
      unchanged (uniqueness invariant is not overridable).

## 2. Daemon API (src/api.rs)

- [ ] 2.1 `create_session` handler: if `req.name` is Some and
      `state.store.session_name_exists(name).await` → return
      `(StatusCode::CONFLICT, Json(ErrorResponse { error: format!("A session
      named '{}' already exists", name) }))` — no session created.
- [ ] 2.2 `rename_session` handler: already maps `Err(msg)` → 409; no change
      needed beyond the store change.

## 3. CLI (src/cli.rs)

- [ ] 3.1 `auto_create_session`: after `send().await?`, inspect `resp.status()`;
      on `409` parse `ErrorResponse` and `fail(json_output, &err.error,
      "SESSION_NAME_TAKEN")`; on other non-success statuses, fail with the
      raw status; on success parse `CreateSessionResponse` as today. (The
      send auto-create fallback passes `name: None` — unaffected.)
- [ ] 3.2 `cmd_session_rename`: 409 arm code `SESSION_ALREADY_NAMED` →
      `SESSION_NAME_TAKEN` (message text already correct from err.error).

## 4. Tests (tests/e2e.rs — write first)

- [ ] 4.1 `test_create_duplicate_name_rejected`: `session create --name
      dup-a` → ok; second `session create --name dup-a` → fails, stderr
      contains "already exists"; `session create --name dup-a2` (different)
      → ok.
- [ ] 4.2 `test_create_duplicate_name_json_error`: `session create --name
      dup-b --json` then duplicate `--json` → exit 1, stderr JSON has
      `"code":"SESSION_NAME_TAKEN"` and error message.
- [ ] 4.3 `test_rename_to_existing_name_rejected`: create `n1` + `n2`;
      `session rename <n1-id> n2` → fails, stderr "already exists";
      rename back `session rename <n2-id> n1` → fails; rename `<n1-id> n1`
      (own name) → still ok (regression guard).
- [ ] 4.4 `test_rename_duplicate_json_error`: `--json` variant asserts
      `"code":"SESSION_NAME_TAKEN"`.
- [ ] 4.5 Regression: `test_send_without_active_session_auto_creates` stays
      green (auto-create with no name unaffected).

## 5. Quality gates

- [ ] 5.1 `cargo fmt --check` clean
- [ ] 5.2 `cargo clippy -- -D warnings` clean
- [ ] 5.3 `cargo test` green (full suite)
- [ ] 5.4 Live-verify on shared daemon: dup create + dup rename + self-rename
      + `--json` paths
