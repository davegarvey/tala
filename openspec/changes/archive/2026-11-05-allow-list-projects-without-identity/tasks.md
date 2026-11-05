# Tasks

## 1. Identity-free project discovery

- [x] 1.1 Make `list-projects` identity-optional in the CLI and store; verify with `cargo test` and the end-to-end test for empty pre-registration listing, listing registered projects without identity, and ambient attention with a supplied identity.
- [x] 1.2 Update command help, README and agent guide to describe project discovery before registration; verify with `tala --help` and review the updated docs.

## Workflow follow-up

- Archive this change after syncing its delta into the main `agent-cli` spec, then verify `openspec validate --all`.
