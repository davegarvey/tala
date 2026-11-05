# Proposal

## Why

Agents can list peers before registering, but cannot list projects even though both commands are read-only, server-wide discovery. Requiring registration blocks agents from choosing whether to join a project before they create an identity.

## What Changes

- Allow `list-projects` to run without a selected agent identity.
- Preserve optional authentication so selected agents still receive the normal ambient attention summary.
- Keep project lifecycle changes restricted to the project owner.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-cli`: project discovery does not require a registered identity.

## Impact

Affected code includes `src/cli.rs` and `src/store.rs`; the end-to-end CLI coverage and `agent-cli` specification will describe the unauthenticated discovery behavior. No wire protocol change is needed.
