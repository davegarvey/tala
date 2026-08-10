# Narrow the CLI surface to the golden path (cycle-19 evals)

## Why

Dave (PO mandate, 2026-08-10): "narrow down the commands and switches; find
the golden path and focus on it." Ran 7 eval scenarios across 2 batches
(golden-path discovery, overlap probe, multisession, 3-agent relay, broadcast
monitor — 11 agents, all feedback + transcripts in /workspace/tala-po/
feedback/cycle-19/). Agent consensus is strong and convergent:

- `stream` — "basically single-session listen; both overlap wait --new-session"
  (7/9 cut or equal-to-listen; zero kept)
- `session list` / `session close` — byte-identical to top-level `list`/`close`
  (4/4 probers cut; `session` group = lifecycle: create/rename/reopen)
- `session show` — list covers it (2/4 cut, no one kept)
- `agents` — "nice-but-late": empty until sessions exist; `discover` lists
  identities pre-session (6/9 cut or clutter)
- `send --name <X>` — implement the cli-cmd-rename design intent (Send gains
  --name): golden-α completed the whole loop with `discover → send → wait →
  send` and never needed `session create`; one-command named start
- `wait --timeout 0` — broken (instant timeout); `listen` documents 0 = no
  timeout. Four agent timeouts this cycle on the fixed 60s default (B048)
- bare `send`/`wait` with multiple open sessions silently targets the active
  one — the biggest cross-topic-send footgun (ms-α, relay-β) → stderr guard
- `--new-session` help misdescribes the flag (it surfaces follow-ups in
  existing sessions too; B049) → fix help/docs, no rename

Kept deliberately: intent flags (--intent/--reply-to/--expect-reply) and
`pending` (9/10 agents; essential for interleaved exchanges), `listen` (only
real-time monitor — its reliability bugs B046 are a separate change),
`check`, `use`, `discover` (essential for multi-party onboarding per relay
agents), `status`, `stop`, `session create/rename/reopen`.

## What Changes

- Remove commands: `stream`, `agents`, `session list`, `session close`,
  `session show` (variants, dispatch, handlers, daemon-compat allow-lists,
  help cross-references, docs, e2e tests).
- Add `tala send --name <NAME>`: creates the session with the given name when
  no session is resolved (the auto-create path already names via
  auto_create_session). Usage error when combined with an explicit session.
- `wait --timeout 0` = wait indefinitely (CLI passes 0 through; daemon treats
  0 as no deadline) — same semantics as `listen --timeout 0`. Applied to
  `wait <session>` and `wait --new-session`.
- Ambiguity guard: bare `send`/`wait` (no explicit session, no --new-session)
  that resolves through the active session prints a stderr warning when more
  than one session is open: "targeting active session <id> (<n> open
  sessions)". Non-blocking; stdout/JSON unchanged.
- Help/doc fixes: `--new-session` flag description (surfaces unread incoming
  from other agents — new sessions first, then participated sessions);
  `--expect-reply` error message names the alternative (`--intent req` or
  `--wait`) when used with req/out (B051); SKILL.md "no auto-create on send"
  corrected (send auto-creates); README command table drops removed commands,
  documents send --name and wait --timeout 0.

## Capabilities

- The golden path is one command shorter: `tala send --name <x> --wait "q"`.
- No silent wrong-session sends: bare commands warn when the active session is
  an ambiguous choice.
- Waits can be indefinite; 0 is consistent across wait and listen.
- Every removed command's help text points nowhere (they're gone); errors for
  removed commands come from clap itself (unrecognized subcommand).

## Impact

- **BREAKING (0.x)**: `stream`, `agents`, `session list`, `session close`,
  `session show` removed. README/changelog notes. SKILL.md and eval scenario
  docs updated (scenarios used only send/wait/history/list/check — minimal
  ripple). e2e suite: ~10 tests removed/reworked, new tests added.
- No wire/daemon changes except timeout-0 handling and the (already merged)
  name in wait-new results.

## Red-team note

Skip formal adversarial review: every cut is backed by multi-agent eval
consensus (matrix in /workspace/tala-po/backlog.md, cycle-19 section), the
PR is the human review gate, and the change is reversible in git. B046
(listen reliability) and B047 (SESSION_NOT_FOUND race) are explicitly OUT of
scope — separate changes.
