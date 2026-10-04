# Tala agent evaluations

Run one scenario manually, collect feedback and CLI transcripts, then review findings. No autonomous loop, generated commits, PRs or merges. Functional integration tests complement these exercises but do not replace real-agent usability feedback.

## Setup

Create scratch projects under a temporary directory and use one isolated TALA_HOME. Resolve and verify the intended binary:

```bash
TALA_BIN="${TALA_BIN:-$(command -v tala)}"
"$TALA_BIN" --version
```

For a checkout evaluation, explicitly set TALA_BIN to the newly built absolute binary path. For ordinary agent communication, use the installed PATH binary. Never mix their homes or daemons.

Each agent registers once and retains its own ID through TALA_AGENT or explicit --agent arguments. Agents may share a project directory: their identities and inboxes must stay separate. Avoid agents sharing the same identity. Give agents only docs/agent-guide.md and their task; observe where they guess, consult help, or bypass the CLI.

## Scenarios

| Scenario | Purpose |
|---|---|
| [Same project](scenarios/same-project.md) | Claude/Codex identities, inboxes and project board in one checkout |
| [Cross project](scenarios/cross-project.md) | Addressed API question, context and automatic reply routing |
| [Delivery and handoff](scenarios/delivery-handoff.md) | Receipt vs answer, timeout, late reply and successor context |

Capture CLI JSON outputs, elapsed times and feedback in scratch files. Retrieve history and pending state through the CLI before stopping the daemon. Record message count, first-reply latency, timeouts, help lookups, incorrect commands, and internal-storage reads. A successful evaluation has correct recipients, no skipped messages, no identity confusion and no storage inspection. Report findings for human review and fold accepted changes into OpenSpec.
