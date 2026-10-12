# Proposal

## Why

Tala's users are coding agents, but parts of its interface still assume a human operator who keeps state between commands, notices new messages and interprets prose. The open issues #93–#98 are symptoms: an agent that sees exit 1 after a stored send will resend (#93); a transport failure reported as a missing session sends it down the wrong recovery path (#98); a sandboxed agent cannot tell a dead daemon from a blocked one (#97); and agents sharing a checkout end up indistinguishable (#94). Several agents commonly work in one directory or in sibling worktrees, and Tala must stay correct there through the CLI alone, without depending on any agent harness's integration points.

## What Changes

- **BREAKING** Agents select their identity with a private handle returned by `register`, not with the public `agt_` ID. Public IDs remain for addressing and appear in transcripts, so copying one can no longer impersonate its owner. `--agent`/`TALA_AGENT` accept handles only.
- **BREAKING** JSON is the default output; `--human` selects the text renderer. `--json` is accepted as a no-op.
- Every error carries a machine-actionable `retry` class, the ID of any message already stored, and one exact `next` command.
- `send --wait` treats transport failures during the wait as transient until its deadline. If the wait still fails after the message is stored, it exits 4 with the stored message ID rather than 1.
- Sends without `--key` derive their idempotency key from sender, target and content. An identical resend within a deduplication window returns the original message instead of a duplicate.
- The CLI distinguishes a refused connection (stale daemon: restart it) from a denied or timed-out one (daemon may be alive but unreachable: do not spawn another; report `DAEMON_UNREACHABLE`).
- Bare `tala` returns an attention summary: incoming requests, unread messages and own outstanding requests, each with the command to act on it, with stale requests collapsed into a count. Ages are reported in seconds.
- Every response from a command run with an identity carries a compact `attention` field, so an agent learns of waiting work as a side effect of any Tala call.
- A project is identified by its repository rather than its worktree, so agents in sibling worktrees share one board.
- Protocol version moves to 3.

Harness hooks, webhooks and push delivery into harness sessions are deliberately excluded; see design.md Non-Goals.

## Capabilities

### New Capabilities
- `agent-attention`: the attention summary, ambient attention on every response, request staleness and age reporting that tell an agent what needs its action.

### Modified Capabilities
- `agent-identity`: identity selection by private handle; explicit upgrade of existing identities.
- `agent-cli`: command surface (bare `tala`), JSON-default output contract, actionable errors and exit code 4, hookless agent instructions.
- `agent-messaging`: content-derived default idempotency keys; blocking send resilient to transient transport failure.
- `durable-daemon`: distinguishing refused from unreachable daemons and never spawning a second daemon when the first may be alive.
- `project-board`: canonical project identity shared across git worktrees.

## Impact

- Code: `src/cli.rs` (output default, bare command, wait loop), `src/client.rs` (error classification, liveness, handle resolution), `src/models.rs` (`Failure` fields, envelope `attention`, `PROTOCOL_VERSION`, new operation), `src/store.rs` (automatic-key window, attention query), `docs/agent-guide.md`.
- Compatibility: agents and scripts using `agt_` IDs with `--agent`, or relying on text output, must update. Existing identities need a one-off handle issued; see design. A protocol 2 daemon must be stopped with its own binary before upgrade.
- Evaluation: new scenarios for interrupted `send --wait`, two agents in one checkout and sibling worktrees, run without hooks for each supported harness. Their results decide whether hooks or push delivery are proposed later.
- Resolves or supersedes #93, #94, #95, #97 and #98; #96 is already fixed by the protocol 2 redesign.
