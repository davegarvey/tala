# Reject unknown flags in `tala send` instead of silently misrouting

## Why

`tala send` silently swallows unknown or typo'd flags and misroutes the message to the
ACTIVE session — or worse, sends the typo'd flag's value as the message content. Verified
on a fresh main build (cycle-07):

- `tala send --new "does --new create a session?"` → exit 0, "Sent message 3 to
  sess_lfuvu" (the active session), no warning, `--new` ignored.
- `tala send --sesion "typo flag test"` / `tala send --timeot "typo timeout"` → exit 0,
  both silently delivered to the active session.
- `tala send --timeot 1` → exit 0, "Sent message 6 to sess_lfuvu" — the typo'd flag's
  **value** ("1") became the message content.
- `tala send --wait "msg" --timeot 1` → cryptic clap error `unexpected argument '1'`,
  exit 2, message silently dropped.

Root cause: `send` positionals use `allow_hyphen_values`, so an unknown `--flag` is
consumed as the SESSION positional; session resolution then fails and the code falls back
to the active session. A typo'd `--sesion`/`--timeot` sends to the wrong session quietly,
and a typo'd flag with a value sends junk content — both exit 0 with no warning. For a
messaging tool where wrong-session delivery is a real trust failure (agents operate on
each other's sessions), silent misrouting is the worst possible failure mode.

## What Changes

- `send` parsing: stop treating unknown `--flags` as the SESSION positional. The SESSION
  positional should only accept session IDs (never hyphen-leading tokens); unknown
  `--flag` becomes a clap usage error (`error: unexpected argument '--timeot' found`,
  exit 2) and **no message is sent**.
- Keep the documented escapes intact:
  - `tala send -- --dashed-content` (explicit `--` separator) still works for dashed
    content.
  - `--stdin` / stdin piping still bypass shell interpretation.
  - `-s/--session <id>` explicit flag still works.
- Because `allow_hyphen_values` is what makes an unknown flag silently consumed, remove
  it from the SESSION positional while keeping the MESSAGE positional permissive for
  single-dash content (e.g. `-` for stdin) — with the `--` separator as the canonical
  escape.
- Warning behavior (B015): the spurious "message starts with '--'" warning currently
  fires even when the documented `--` separator was used. With the new parsing the
  warning should only fire for content that actually starts with `--` in an ambiguous
  position (i.e. when the user did NOT use the `--` separator).

## Capabilities

### New Capabilities
- `send-flag-validation`: unknown/typo'd `--flags` on `tala send` fail loudly (exit 2,
  clap usage error) with the message unsent; no silent fallback to the active session.

### Modified Capabilities
- `send`: session-positional parsing no longer accepts hyphen-leading tokens; message
  content with leading dashes requires the `--` separator, `--stdin`, `--message-file`,
  or stdin piping (documented).
- `send-error-reporting`: failure modes now distinguish "unknown flag" (exit 2) from
  "session closed" (exit 1) from "no active session" (exit 1) — no silent success.

## Impact

- `src/cli.rs` — `send` subcommand arg definitions (`allow_hyphen_values` on SESSION
  positional), warning condition for leading-`--` content.
- `tests/e2e.rs` — new tests: unknown flag → exit 2 + no message sent; `--` separator
  still works; `-s` explicit flag unaffected; typo'd flag with value → no junk message.
- `.opencode/skills/tala/SKILL.md` + `.opencode/commands/tala.md` — document the
  `--`-separator rule for dashed content (already partially documented; align wording).
- `openspec/changes/reject-unknown-send-flags/tasks.md` — task list.

## Backlog references

- B026 (send swallows unknown flags → misroutes to active; confirmed cycle-06 + cycle-07),
  B015 (spurious `--`-warning when separator used; fixed as part of the parsing change).
