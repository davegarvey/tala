## 1. Closed-Session Send UX

- [x] 1.1 In `cmd_send()`, improve the `SESSION_CLOSED` error message to include the session ID and suggest `tala session reopen <id>` or `tala start`
- [x] 1.2 In the stale active-session detection path, check if the session is closed and handle it with a clear error + clear stale active session

## 2. Wait Feedback and Timeout

- [x] 2.1 In `cmd_wait()`, update initial feedback messages to include the timeout value (e.g., "Waiting for messages in session X (timeout: Ys)...")
- [x] 2.2 Reduce default timeout from 300s to 60s in `read_user_config()` default value
- [x] 2.3 Reduce default timeout from 300s to 60s in API `wait_for_message()` fallback

## 3. unread_count Bug Fix

- [x] 3.1 In `compute_session_unread()`, fall back to `get_default_sender()` when `read_project_config()` returns `None`

## 4. CLI Help Clarity

- [x] 4.1 Update `Wait` command help text to clarify when to use wait vs stream vs listen
- [x] 4.2 Update `Stream` command help text with cross-references
- [x] 4.3 Update `Listen` command help text with cross-references
