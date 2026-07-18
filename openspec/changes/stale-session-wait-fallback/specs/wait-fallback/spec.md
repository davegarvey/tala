## MODIFIED Requirements

### Requirement: `chit wait` handles stale active session gracefully

When `chit wait` receives a `SESSION_NOT_FOUND` error from the daemon for the active session, it SHALL NOT hard-error. Instead, it SHALL:
1. Clear the stale active session via `store::clear_active_session()`
2. Fetch available sessions from the daemon
3. Fall back based on available sessions (same logic as the existing no-active-session path in `cmd_wait` lines 916-1003):
   - If no active sessions: wait for a new session
   - If exactly one active session: use it
   - If multiple: wait-all across sessions
4. Continue the wait operation with the resolved session

#### Scenario: Stale active session, one active session exists
- **GIVEN** `.chit/active-session` contains a stale session ID `sess_stale`
- **AND** the daemon has one active session `sess_abc`
- **WHEN** user runs `chit wait`
- **THEN** the stale active session is cleared
- **THEN** `chit wait` uses `sess_abc` and waits for new messages

#### Scenario: Stale active session, no active sessions exist
- **GIVEN** `.chit/active-session` contains a stale session ID `sess_stale`
- **AND** the daemon has no active sessions (all closed or none)
- **WHEN** user runs `chit wait`
- **THEN** the stale active session is cleared
- **THEN** `chit wait` SHALL wait for a new session

#### Scenario: Stale active session, multiple active sessions exist
- **GIVEN** `.chit/active-session` contains a stale session ID `sess_stale`
- **AND** the daemon has multiple active sessions (`sess_abc`, `sess_def`)
- **WHEN** user runs `chit wait`
- **THEN** the stale active session is cleared
- **THEN** `chit wait` SHALL wait across all active sessions, printing messages with session IDs and returning (wait-all behavior)
