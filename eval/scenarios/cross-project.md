# Cross-project agents

Use separate scratch frontend/API Git checkouts and one isolated Tala home. Give each agent docs/agent-guide.md and an independently registered identity.

A (frontend): Ask API agent whether an endpoint accepts null or requires omission. Include enough context to answer without reading the frontend checkout. Wait for the correlated answer and record the decision.
B (API): Receive the question, inspect relevant fixture source in your project, and reply with the contract and evidence. Do not edit the other project.

Measure discovery/first-reply latency, help lookups, wrong commands, timeouts and storage reads. Expected: explicit recipient, originating checkout context, correctly correlated response, no unrelated inbox consumption. Dump history through the CLI and collect feedback.
