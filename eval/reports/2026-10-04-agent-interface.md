# Agent interface evaluation — 4 October 2026

Two independently prompted subagents used the built Tala 0.34.0 binary and only the agent guide/help. They first collaborated in one scratch directory, then registered separate frontend/API instances in different directories, sharing one isolated Tala home. The tool labels were codex/claude; both evaluators ran in this Codex session. This does not verify a live Claude integration or external notification hooks.

| Measure | Same project | Cross project |
|---|---:|---:|
| Direct messages | 2 | 2 |
| Request-to-reply time | 21 s | 21 s |
| Receipt lag at timestamp precision | <1 s | <1 s |
| CLI errors / wait timeouts | 0 / 0 | 0 / 0 |
| Internal-storage reads | 0 | 0 |
| Pending requests after reply | 0 | 0 |

The first exchange identified the parser fixture's quoted-comma bug and supplied a csv.reader correction. The second correctly distinguished omitted customer name from explicit null using the API fixture. Both replies retained the request thread and correct destination. Shared board announcement/completion were visible to both instances. JSON history exports contained two messages per direct thread; board export contained two posts. Requester consulted top-level help and send help; responder consulted top-level help. Peer discovery required one additional listing while registration was still underway.

## Findings addressed

- Successful `send --wait` initially returned a stale send-time request alongside its answer. It now returns the request state atomically observed when consuming the reply; timeout keeps the original acknowledgment labeled `snapshot: at_send`. Regression assertions verify both shapes.
- The guide/help now explicitly states that `--wait` consumes only the correlated reply, explaining why a subsequent inbox is empty.
- The guide now states that changing CWD does not change an existing agent's registered project/default board.

Epoch timestamps were usable but ISO timestamp presentation was suggested as an optional convenience. No extra timestamp fields were added.

## Evidence and limits

Original feedback and CLI JSON exports remain under `/tmp/tala-agent-eval.v5avjQ/feedback/`: requester.md, responder.md, same-project-history.json, cross-project-history.json, shared-board.json. Both agents unregistered their own identities. The orchestrator checked the isolated daemon after evidence collection and confirmed it was no longer running. Several daemon restarts occurred between tool invocations; durable state retained the exchanges.

This is a small usability sample, not a throughput benchmark. The 21-second timings include agent reasoning and tool orchestration. Automated tests separately cover concurrent startup/consumption, inbox replay after a lost response, crash/restart durability, visibility, retry conflicts, corrupt/write-failing storage, timeout/late replies and integration failure preservation. macOS checks ran locally; Linux and Windows checks are configured in CI and remain to be verified there.
