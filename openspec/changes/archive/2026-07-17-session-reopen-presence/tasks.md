## 1. DaemonEvent model

- [x] 1.1 Add `SessionReopened(String)` variant to `DaemonEvent` enum in models.rs

## 2. Session reopen (daemon + API)

- [x] 2.1 Add `reopen_session` method to Store: sets `closed = false`, updates `last_activity`, broadcasts `DaemonEvent::SessionReopened`
- [x] 2.2 Add `POST /api/sessions/:id/reopen` endpoint in api.rs
- [x] 2.3 Add `chit session reopen <id>` clap command and handler with `--json`/`-j` support

## 3. `chit use` on closed session error message

- [x] 3.1 In `cmd_use`, query `GET /api/sessions/:id` to check session closed status before setting active
- [x] 3.2 Show "Session '<id>' is closed. Use `chit session reopen` to continue" when closed

## 4. `chit close --quiet`

- [x] 4.1 Add `--quiet`/`-q` flag to `Close` clap args
- [x] 4.2 Gate human-readable confirmation on `!quiet`; JSON output is never suppressed

## 5. `chit stream` alias for `chit follow`

- [x] 5.1 Add `#[command(alias = "stream")]` to `Follow` command struct

## 6. Tests

- [x] 6.1 Add tests: reopen closed session, send to reopened session, reopen already-open, reopen nonexistent, reopen --json
- [x] 6.2 Add tests: `chit close --quiet`, `chit close --quiet --json`, `chit use` on closed session
- [x] 6.3 Verify `chit stream` works as `chit follow` alias
