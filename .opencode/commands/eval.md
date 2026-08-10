---
description: Run a tala evaluation scenario with real agents to gather feedback and improve the product.
---
Run a tala eval scenario. Read `eval/README.md` for the orchestration steps and guardrails, and `eval/scenarios/<name>.md` for the specific scenario (setup commands, seed files, agent prompts, metrics, baseline).

Choose the scenario by what you want to evaluate:

- `cross-project` — general two-agent collaboration (send/wait/history/intents)
- `intent-protocol` — intent metadata: `--intent`, `--reply-to`, `pending`
- `wait-deadlock` — waiting visibility and deadlock prevention

Important: this is a manually orchestrated eval — no autonomous loop, no commits, no PRs. Run the scenario in a scratch dir, collect feedback and transcripts, triage findings, then fold accepted findings into an OpenSpec change proposal for human review.
