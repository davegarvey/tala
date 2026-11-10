# Proposal

## Why

The main specs lag behind the code after two changes. `output-format-flag` made `--format=text` the documented text option, but scenarios in other requirements still name `--human`. `lan-network-access` added behaviours that the specs do not describe: status in remote mode reports identity context as unavailable instead of reading a local database, and `enable-network` starts the daemon when none is running. Specs should describe the system as built.

## What Changes

- Replace `--human` with `--format=text` in agent-cli and agent-attention requirements, except where a scenario tests the alias itself.
- Specify status identity context in remote mode (agent-cli, durable-daemon).
- Specify that `enable-network` starts a daemon when none is running, when it reports `restart_required`, and the order of suggested URLs (network-access).

No code changes; the behaviour is already implemented and tested.

## Capabilities

### Modified Capabilities

- `agent-cli`, `agent-attention`, `durable-daemon`, `network-access`.

## Impact

Specification text only.
