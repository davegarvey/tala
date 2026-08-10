# Enforce JSON error contract for `--json` commands (B031) + honest "Nothing to send" hint (B030)

## Why

`--json` is the documented structured-output mode: "Every command supports
--json for structured output." Automation (agents scripting tala) parses
stdout/stderr expecting JSON. But error paths are inconsistent:

1. **B031 — `--json` error paths are not JSON.** `tala send --json` with
   nothing to send prints the human block `Error: Nothing to send. …` to
   stderr, exit 1 — while `tala send --json <closed-sess> "msg"` correctly
   emits `{"code":"SESSION_CLOSED","error":…}`. Same command, two error
   contracts. Any agent that parses `--json` output breaks on the first error
   path. Root cause: cmd_send's "Nothing to send" and "Message cannot be
   empty." paths use `anyhow::bail!` instead of the existing
   `fail(json, msg, code)` helper (src/cli.rs:817, 848) that every other error
   path uses.
2. **B030 — misleading "Active session:" hint.** The "Nothing to send" hint
   labels the first OPEN session as "Active session:" even when no session is
   active in this project — cycle-10 repro: in a brand-new project dir it
   labeled `sess_50y63`, which is alpha's active session in a DIFFERENT
   project dir. An agent following the hint would `use` a session that isn't
   theirs. It must say "open session" (or nothing), never "Active session:".

## What Changes

- `cmd_send`'s two `anyhow::bail!` sites become `fail(json_output, msg, code)`:
  - no content + no session → `fail(json, "Nothing to send. …", "NOTHING_TO_SEND")`
  - empty message → `fail(json, "Message cannot be empty.", "EMPTY_MESSAGE")`
- The hint in the "Nothing to send" path: relabel `Active session: …` →
  `Open session: …` (accurate: it is the first open session, not the active
  one) and keep the guidance (`tala use` / `--session`).
- Human (non-json) output unchanged: same "Error: …" text as today.
- JSON error shape matches the existing helper: `{"error": "<msg>", "code": "<code>"}`
  on stderr, exit 1.

## Capabilities

- `tala send --json` (no content) → `{"error":"Nothing to send. …","code":"NOTHING_TO_SEND"}`, exit 1.
- `tala send --json ""` → `{"error":"Message cannot be empty.","code":"EMPTY_MESSAGE"}`, exit 1.
- `tala send "msg"` (no session, human mode) → unchanged human error + honest
  "Open session: …" hint, exit 1.
- All other `--json` error paths already route through `fail()` — unchanged.

## Impact

- Files: src/cli.rs (two error sites + hint text), tests/e2e.rs (new tests).
- No API surface change; daemon untouched.
- Behavior change is strictly on error paths; success paths untouched.
- Fixes backlog B031 (promoted P3→P1: breaks every --json consumer) and B030 (P3).
