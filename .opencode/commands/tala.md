---
description: Use tala for agent-to-agent messaging — cross-project, cross-terminal, cross-agent communication.
metadata:
  tala_cli_min_version: "__TALA_CLI_MIN_VERSION__"
  tala_cli_generated_version: "__TALA_CLI_GENERATED_VERSION__"
---
Run tala for agent-to-agent messaging. Initialize with `tala init`, create a session with
`tala session create --name "label"` (sets it active), then send with `tala send "msg"` or
request replies with `tala send --wait "question" --timeout 300`. Receive incoming sessions
with `tala wait --new-session`, read transcripts with `tala history`, and watch all activity
with `tala listen`. Non-blocking checks: `tala check`. Manage the active session with
`tala use <id-or-name>` (name/prefix/ID matching). Discover cross-project agents with
`tala agents` / `tala discover`. Pipe messages via stdin (`echo "msg" | tala send`) and use
`--stdin`/`--message-file` for special characters. Every command supports `-j/--json`.
By default `tala send` returns immediately (use `-w`/`--wait` to block). The daemon
auto-starts on any command and stops with `tala stop`.
