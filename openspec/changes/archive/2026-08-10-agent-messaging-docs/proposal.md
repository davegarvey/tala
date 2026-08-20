# Agent messaging docs — golden path, input methods, intents (#65)

## Why

Issue #65 (Dave, 2026-08-09): README shows only inline-argument examples;
agents composing multi-line/structured messages hit quoting pain with argv
and only discover `--stdin`/`--message-file` by reading help carefully.

Eval cycle-18 evidence (2026-08-10): both eval agents independently found
safe input methods (`--stdin` heredoc, `--message-file`) only because `send
--help` hints at them — alpha: "I assembled the flow from four help screens;
documentation could show one canonical end-to-end example up front."
Also unaddressed: intent precedence (`--wait` vs `--reply-to` combined) is
undocumented (beta avoided combining them), and the README command table is
stale (missing `pending`, `use`, `session` group, intent flags).

## What Changes

README.md (user-facing) + `.opencode/skills/tala/SKILL.md` (agent-facing):

- **Canonical agent handshake** example: `init` → `session create --name` →
  `send --intent req --wait` → (peer) `wait --new-session` → `send --intent
  reply --reply-to <id>` → `pending` → `close`.
- **Input-method decision guidance** (issue #65): one-line plain → inline
  argv; multi-line/backticks/`$`/quotes → piped quoted heredoc (`--stdin` if
  disambiguation needed); draft-then-edit → `--message-file`; structured
  payloads → `--part`. Compact table.
- **Intent precedence** (F2): explicit `--intent` wins; `--reply-to` implies
  `reply`; `--wait` implies `req`; `--reply-to` + `--wait` = reply that also
  expects a reply. Re-ask pattern: `--reply-to <orig> --intent req`.
- **Command table refresh**: add `pending`, `use`, `session` group; intent
  flags on `send`/`wait` rows.

## Capabilities

- An agent can learn the complete golden path from README alone without
  assembling it from help screens.
- Input-method choice is a documented decision, not a shell-quoting gamble.
- Intent semantics (incl. precedence) are stated, not inferred.

## Impact

- Docs only (`README.md`, `SKILL.md`); no code, no tests, no wire change.
- `--help` texts stay the source of truth for flag details; README links the
  narrative.

## Red-team note

Docs-only; content is grounded in eval transcripts (cycle-18) and #65.
PR is the human review gate.
