## Why

Two observe eval rounds with 4 agents each revealed consistent friction in chit's session management and CLI ergonomics. The top complaints: sessions are hard to identify, `chit start` doesn't set the active session causing message misrouting, session names are invisible in default output, and several command behaviors don't match user expectations or documentation.

## What Changes

- `chit start` now sets the active session after creating it (reversing previous change based on eval feedback)
- `chit init` accepts an optional positional name argument (`chit init my-project`) in addition to the existing `--name` flag
- `chit send` with no session and no active session no longer silently creates a new session — it lists available sessions and suggests `chit use`
- `chit list` shows session names in the default human-readable output (not just `--json`)
- `chit use` accepts session names (not just opaque IDs), resolving to the matching session
- `chit session rename` success message no longer renders JSON quotes around the name
- `chit observe --timeout <secs>` actually terminates after the given timeout (previously ignored)
- Auto-created sessions (from `chit send`) inherit the project name from `.chit/config.json` as their session name

## Capabilities

### New Capabilities
- `cli-ergonomics`: Positional name arg for `chit init`, name-based lookup for `chit use`, session names in `chit list` default output

### Modified Capabilities
- `session-lifecycle`: `chit start` sets active session. `chit send` no longer auto-creates sessions. Auto-created sessions get named from project config. `chit session rename` quoting fixed.
- `message-observation`: `chit observe --timeout` now functions correctly

## Impact

- `src/cli.rs` — `cmd_start` (add active session write), `cmd_send` (fail instead of auto-create, use project name), `cmd_init` (positional arg), `cmd_use` (name resolution), `cmd_list` (show names), `cmd_session_rename` (fix quoting), `cmd_observe` (use timeout parameter)
- `src/api.rs` — potentially new endpoint for session name→ID resolution
- `src/store.rs` — potentially a lookup-by-name helper
- `tests/e2e.rs` — updated tests for new behaviors
- `.opencode/skills/chit/SKILL.md` — update `chit init` and `chit use` docs
