# Cycle-18 eval feedback fixes (B039–B041)

## Why

Stint-2 eval cycle-18 (2026-08-10) ran two scenarios (cross-project,
intent-protocol) with real sub-agents on v0.28.0. Transcripts and honest
feedback surfaced three small, evidence-grounded defects in the core agent
loop:

- **B039** (P1, cross-project alpha): `tala session create --json` exits 0
  with **no output at all** on success. Alpha re-ran the command and only a
  `SESSION_NAME_TAKEN` error revealed the session had been created. `--json`
  mode should never be silent on success — every other command emits a typed
  document (B031 fixed the error paths; this is the success path).
- **B040** (P3, cross-project beta): `send --intent reply` without
  `--reply-to` is accepted silently, making the reply uncorrelated. Beta:
  "easy to forget the id". A stderr warning keeps the intent model honest.
- **B041** (P3, cross-project beta): `wait --new-session` result carries only
  the session id; beta had to run `history` to learn the session name.
  `--json` should include the name; text mode must keep printing the bare id
  on stdout (agents capture it via `sess=$(tala wait --new-session)`) and
  may print the name as context on stderr.

## What Changes

- **`src/cli.rs` `cmd_session_create`**: in `--json` mode, print
  `{"session_id": "<id>"}` on success (text mode already prints the id).
  Fix lives in the command, not in `auto_create_session`, so `send`'s
  auto-create path keeps emitting exactly one JSON document.
- **`src/cli.rs` `send_message`**: when the resolved intent is `reply` and no
  `--reply-to` was given, print a one-line warning to stderr (never stdout;
  JSON output stays pure).
- **`src/api.rs` `wait_new_stream`**: include the session `name` in the
  terminal `result` event (all four delivery sites), looked up via
  `store.get_session`. `src/cli.rs` `cmd_wait_new`: `--json` passes the name
  through; text mode prints the bare id to stdout and the name to stderr.

## Capabilities

- `tala session create --json` emits a typed success document (consistent
  with `{"session_id": ...}` used by `use`, `wait --new-session`).
- `tala send --intent reply` without correlation warns on stderr.
- `wait --new-session --json` includes `"name"`; text output contract
  unchanged (bare id on stdout, name as stderr context).

## Impact

- CLI + daemon only; no wire-format break (added optional field).
- Three e2e tests (one per fix), fmt + clippy clean, live verification
  against a fresh daemon before PR.

## Red-team note

Skip formal adversarial review: changes are <60 lines total, each grounded in
a specific eval transcript finding, and the PR is the human review gate.
Backlog IDs B039–B041; evidence in `/workspace/tala-po/backlog.md`.
