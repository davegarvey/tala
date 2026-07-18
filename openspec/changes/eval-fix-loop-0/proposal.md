## Why

The cross-project eval scenario revealed two P0 issues (session reopen silently fails, TALA_HOME misdirected error messages), three P1 issues (listen timeout, non-persistent session rename, misleading daemon stop messages), and several P2 ergonomic gaps. These undermine the core session model and degrade the developer experience, especially on macOS.

## What Changes

- Fix session reopen lifecycle so `tala session reopen` correctly marks sessions as open and subsequent sends/close work
- Fix daemon resolution error messages to check `$TALA_HOME` first before falling back to `~/.tala/`
- Add default timeout (`--timeout 300`) to `tala listen` so it doesn't block indefinitely on macOS
- Fix session rename persistence — rename should be persisted in the store and not revert on counterparty messages
- Improve `tala stop` message when daemon is already stopped
- Add `tala session close` subcommand for symmetry with `tala session reopen`
- Clarify help text for `tala listen` vs `tala stream`, `--file` flag discoverability
- Improve help text for `tala init` to clarify its purpose

## Capabilities

### New Capabilities
- `session-rename-persistence`: Session names survive daemon restarts and are not overwritten by counterparty messages

### Modified Capabilities
*(No existing spec files to modify — no previous capabilities defined)*

## Impact

- **src/cli.rs**: Add default timeout to listen, add session close subcommand, improve help text, fix daemon stop message
- **src/api.rs**: Fix reopen endpoint, fix rename endpoint for persistence
- **src/store.rs**: Fix TALA_HOME error messages, persist session names to disk
- **src/models.rs**: Any changes to session data structures for rename persistence
- **tests/e2e.rs**: Tests for reopen, rename persistence, listen timeout
