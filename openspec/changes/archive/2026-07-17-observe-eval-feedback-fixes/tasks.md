## 1. Session Lifecycle

- [x] 1.1 `chit start` sets active session — add `store::write_active_session(&session.id)` in `cmd_start` after session creation
- [x] 1.2 `chit start` auto-names session from project config — read `store::read_project_config()` and pass as `name` in `CreateSessionRequest` when no `--name` provided
- [x] 1.3 `chit send` no longer auto-creates on missing session — replace the auto-create block (lines 516-527) with an error listing active sessions
- [x] 1.4 `chit send` auto-creates with project name on stale session — in the stale-session replacement path (lines 502-514), pass project name as session name
- [x] 1.5 `chit session rename` quoting fix — change `result["name"]` to `result["name"].as_str().unwrap_or("")` in success message

## 2. CLI Ergonomics

- [x] 2.1 `chit init` positional name arg — add `name: Option<String>` as positional arg to `Init` variant, error if both positional and `--name` provided
- [x] 2.2 `chit list` shows session names — modify the default output format to include `s.name.as_deref().unwrap_or("-")` alongside the ID
- [x] 2.3 `chit use` accepts session names — add name resolution logic: fetch active sessions, filter by exact name match, error on ambiguous/missing (with fallback to ID-based matching if no name match found), then call `write_active_session`

## 3. Message Observation

- [x] 3.1 Add `timeout_secs: Option<u64>` to `ObserveParams` struct in api.rs
- [x] 3.2 Implement server-side timeout in `observe_events` — use `tokio::select!` via `tokio::time::timeout` wrapping `rx.recv()` when timeout is set
- [x] 3.3 Wire CLI `_timeout` to `timeout_secs` query parameter — remove underscore, pass to `/api/observe`

## 4. Docs & Tests

- [x] 4.1 Update `.opencode/skills/chit/SKILL.md` — fix `chit init <name>` syntax, add `chit use <name>` example
- [x] 4.2 Update e2e tests for `chit start` active session behavior
- [x] 4.3 Add e2e tests for `chit use` by name
- [x] 4.4 Add e2e tests for `chit send` no-auto-create behavior (both plain and JSON error output)
- [x] 4.5 Add e2e tests for `chit observe --timeout`
- [x] 4.6 Add e2e tests for `chit init` positional name and conflict with `--name`
