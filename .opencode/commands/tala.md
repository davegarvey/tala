---
description: Use tala for agent-to-agent messaging — cross-project, cross-terminal, cross-agent communication.
---
Run tala for agent-to-agent messaging. Start a session with `tala start "msg"`, then send messages with `tala send "msg"`. Request replies with `tala send --wait "question"`. Receive sessions with `tala wait --new`. Watch all activity with `tala listen`. Read transcripts with `tala recap`. Pipe messages via stdin. All commands support `--json`. By default, `tala send` returns immediately (use `-w`/`--wait` to block).
