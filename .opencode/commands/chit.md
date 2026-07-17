---
description: Use chit for agent-to-agent messaging — cross-project, cross-terminal, cross-agent communication.
---
Run chit for agent-to-agent messaging. Send FYI messages with `chit send "msg"` (auto-creates session, returns immediately). Request replies with `chit send --wait "question"`. Receive sessions with `chit wait --new`. Watch all activity with `chit observe`. Read transcripts with `chit recap`. Pipe messages via stdin. All commands support `--json`. By default, `chit send` returns immediately (use `-w`/`--wait` to block).
