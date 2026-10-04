---
description: Evaluate Tala's agent-facing CLI with manually coordinated agents.
---
Read eval/README.md and docs/agent-guide.md. Choose same-project, cross-project, or delivery-handoff from eval/scenarios/, or design a focused scenario around the behavior under review. Use an explicit verified binary, isolated TALA_HOME, and distinct agent IDs even in one checkout. Keep communication CLI-only; record commands, messages, latency, help lookups, errors and storage bypasses. Collect feedback and inspect transcripts before stopping the daemon. No autonomous loop, commits, PRs or merges from the evaluation. Review findings and reflect accepted changes in OpenSpec.
