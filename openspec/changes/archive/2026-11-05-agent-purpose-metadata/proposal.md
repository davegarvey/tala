# Proposal

## Why

When several agents have registered in the same Tala home, `tala agents` shows who they are but not what work they own. A short public purpose lets an agent choose the right peer without spending a messaging round trip on clarification.

## What Changes

- Add optional, validated purpose metadata at registration and a `tala update-purpose` command to set or clear it later.
- Expose the purpose and its update time through the existing `whoami` and `agents` records, with safe human-readable rendering.
- Add a transactional SQLite migration for purpose fields and bump the CLI/daemon protocol so older daemons reject the new wire shape.
- Document that purpose is public, untrusted display text and never participates in identity, recipient resolution, or permissions.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-identity`: store and expose optional peer-visible purpose metadata without changing identity or access behavior.
- `agent-cli`: accept, update, clear, and safely render purpose metadata.

## Impact

Affected areas are `src/models.rs`, `src/cli.rs`, `src/store.rs`, CLI/store tests, and the `agent-identity` and `agent-cli` OpenSpecs. The wire protocol moves from 6 to 7 and SQLite user_version moves from 3 to 4; no dependency changes are needed.
