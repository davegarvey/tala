## Why

The cross-project eval critic identified five actionable issues: `tala start` has no `--wait` flag, `tala wait` doesn't update the cursor (breaking subsequent `tala whatsup` calls), `tala init` rejects `--json` despite the help text claiming all commands support it, command aliases like `send`/`chat` are hidden from help, and `tala init` requires a separate command to start the daemon.

## What Changes

- Add `--wait` / `-w` flag to `tala start` so users can create a session and wait for the first reply in one step
- Update `tala wait` to persist the cursor after receiving messages, fixing `tala whatsup` reporting "No new messages" when messages exist
- Add `--json` flag to `tala init` for consistent scripting support
- Surface the `send`/`chat` alias in help text for discoverability
- Auto-start the daemon during `tala init` so setup is a single command

## Capabilities

### New Capabilities
- `start-wait`: `tala start --wait` flag to create a session and block until the first reply arrives
- `cursor-tracking`: Cursor persistence in `tala wait` so `tala whatsup` correctly reports new messages
- `init-improvements`: `--json` flag and daemon auto-start for `tala init`

### Modified Capabilities
- `help-clarity`: Command alias (`send`/`chat`) surfaced in help text for discoverability

## Impact

- `src/cli.rs`: Add `--wait` flag to Start command; add `write_cursor()` call in `cmd_wait`; add `--json` flag to Init command; update help text aliases; add daemon start to `cmd_init`
- `src/store.rs`: No changes needed (cursor functions already exist)
