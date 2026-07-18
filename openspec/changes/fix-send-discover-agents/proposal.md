## Why

Three P1 issues were identified during cross-project eval: `tala send --file -` errors instead of reading piped stdin (contradicting documented convention), `tala discover` reports daemon status as "stopped" for running daemons (eroding trust in discovery), and `tala agents` does not show agents until after at least one message has been sent (blocking pre-coordination awareness). Several P2 polish issues also need attention: self-messages counted as unread, inconsistent cursor/since terminology, and sparse `tala use` output.

## What Changes

- Fix `tala send --file -` to read from stdin when `-` is given as filename
- Fix `tala discover` to correctly detect running daemons via direct port probing instead of relying solely on `/api/agents` endpoint
- Fix `tala agents` to show agents from discovered projects and session participants even before messaging
- Fix own messages not being excluded from unread/new-message counters
- Add `--cursor` as accepted alias for `--since` on recap and wait commands for consistency
- Enhance `tala use` output to include session name and message count
- Fix `tala chat` and `tala send` documentation consistency

## Capabilities

### New Capabilities
- `discovery-reliability`: Correct daemon status detection and agent visibility in cross-project discovery

### Modified Capabilities
- `send-receive`: `--file -` must read from piped stdin; own messages must not increment unread counters
- `session-management`: `tala use` output must include session name and message count
- `agent-discovery`: `tala agents` must show discovered-but-unmessaged agents

## Impact

- `src/cli.rs`: Changes to `cmd_send`, `cmd_discover`, `cmd_agents`, `cmd_use`, `compute_session_unread`, `cmd_recap`, `cmd_wait`
- `src/api.rs`: Changes to `/api/agents` handler for agent derivation
- `src/models.rs`: Potential changes to query/response types
- `src/store.rs`: Potential changes to cursor logic for self-message exclusion
