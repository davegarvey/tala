# Design

## Context

See proposal.md for motivation. The relevant current state:

- Bare `tala` already returns a bounded attention summary without acknowledging anything, and `--agent-file` / `--save-handle-to` already provide private selector files. The hook runner needs no new daemon operation.
- `init` writes `.tala/AGENTS.md` and OpenCode skill and command documents. The `agent-cli` spec forbids it from touching harness configuration, and this change keeps that rule for `init`.
- Hook mechanisms, as documented in November 2026:
  - **Claude Code** (CLI and the desktop Code tab): command hooks in `~/.claude/settings.json` or `.claude/settings.local.json`. `PostToolUse` and `SessionStart` (matchers `startup`, `resume`, `clear`, `compact`) receive JSON on stdin including `session_id` and `cwd`. Context is added through `hookSpecificOutput.additionalContext`.
  - **Codex** (CLI and desktop app): `~/.codex/hooks.json` or `.codex/hooks.json` (the project layer loads only when trusted). The same `PostToolUse` / `SessionStart` names and `hookSpecificOutput.additionalContext` shape. `SessionStart` reports `source` (`startup`, `resume`, `clear`, `compact`, `fork`). Plain stdout is ignored for `PostToolUse`. Hooks run only after the user reviews them (desktop: Settings → Hooks; CLI: `/hooks`).
  - **OpenCode**: TypeScript/JavaScript plugins in `~/.config/opencode/plugins/` or `.opencode/plugins/`, with `tool.execute.after` (whose `output.output` string can be changed) and session events. Whether a change to `output.output` reaches the model, and which event can add context at session start and after compaction, are not documented authoritatively. Task 1 verifies both.
- GUI applications on macOS do not inherit the login shell's `PATH`, so a hook command `tala` may not resolve from a desktop app.

## Goals / Non-Goals

**Goals:**
- Notify a busy agent of new requests and unread messages at its next tool call.
- Recover the selector path after compaction.
- Keep each connector a data directory that can be added without changing messaging code.

**Non-Goals:**
- Durable lifecycle events (acceptance, decline, deadline, supersession, delivery-overdue) in hook output. These stay with `tala wait`; adding them would need a non-consuming event read for hooks and is a separate change.
- Waking an idle agent. Background `tala wait` remains the mechanism where a harness supports it.
- Connectors for chat-only clients (Claude Chat, ChatGPT). They have no tool-call hooks; a `tala mcp` server is a separate proposal.
- Loading connector manifests from outside the binary. A third-party manifest would run commands inside a user's harness; until there is demand, new connectors arrive through the catalogue.
- Automatic registration from a hook. Registration stays an explicit agent action.

## Decisions

### Connectors are embedded data, with one generic runner
Each connector lives in `connectors/<name>/` as a `connector.json` manifest plus templates, embedded with `include_str!` in the same way as the guide. The manifest declares:
- `harness` and supported `scopes`, each with a target path;
- `install`: either a JSON merge into a configuration file (Claude Code, Codex) or a file written from a template (OpenCode);
- `session_key`: the JSON pointer for the session identifier in the hook payload (`/session_id`), and `start_source` for the session-start source (`/source`);
- `render`: how to emit text, either as raw stdout or wrapped in a JSON object at a declared pointer with fixed fields (for example `hookEventName`).

`tala connector hook <name> <event>` reads the payload, applies the manifest and calls shared code for selector resolution, attention and rendering. The alternative was per-harness Rust modules. They are simpler for the first connector, but harness knowledge would spread through the binary, and the third connector would show whether the format generalises only after the code had already diverged. Claude Code and Codex share a payload and output shape, so two of the three connectors differ only in paths and labels, which is evidence that a declarative format fits.

JSON rather than TOML avoids a new dependency. Tala writes only JSON configuration, because Codex accepts `hooks.json` as well as `config.toml`.

### Ownership marking for install and remove
Every hook entry Tala writes has a command beginning with the installing binary's absolute path followed by `connector hook <name>`. Remove deletes exactly the entries whose command matches `connector hook <name>`, regardless of binary path, so it also cleans up stale installations. For OpenCode the plugin file is wholly owned: it starts with a generated-file header recording the connector and Tala version, and remove deletes it only if that header is present. Writes reuse `init`'s staged-write-and-rollback approach.

### Absolute binary path
Hooks call the absolute path of the installing executable (`std::env::current_exe`, canonicalised), because desktop apps may not resolve `tala` from `PATH`. The project's documented upgrade replaces the binary in place, so the path survives upgrades. `check` reports a stale installation when the path is missing or the recorded version differs from the running binary's.

### Default scope is user
Identity is per session and the runner is silent outside initialised projects and unregistered sessions, so one user-scope install covers every project in both desktop apps without committing files. Project scope writes to `.claude/settings.local.json` (git-ignored by Claude Code convention), `.codex/hooks.json` or `.opencode/plugins/tala.ts`, for users who want it limited to one repository.

### Session state under the Tala home
`$TALA_HOME/sessions/<connector>/<key>/` (mode 0700) holds `selector`, which the agent creates with `register --save-handle-to`, and `notified.json`, which the runner owns. Keys are the harness session identifier, with characters outside `[A-Za-z0-9_-]` percent-encoded and truncated with a hash suffix beyond 128 bytes. The runner creates the directory before injecting a registration instruction, because `--save-handle-to` requires an existing private directory. Session directories persist after a session ends, as selector files written with `--save-handle-to` do today. Each holds the handle of one agent, which can be unregistered. Pruning old session state is left to a later change if it proves to be a problem.

### Change detection on the client
`notified.json` records the open request IDs and unread message IDs last reported, with timestamps. The runner reports when the current summary contains an ID not in that set, or when a reported ID has remained open for 15 minutes since it was last reported. This keeps the daemon and wire protocol unchanged. The alternative, a daemon-side "last notified" cursor, would need a protocol bump for a value that only the hook uses. The attention summary lists at most three requests, so detection over the summary is bounded. Unread messages are compared by the IDs returned in the summary's first page.

### The runner never starts a daemon
The runner reads daemon metadata and connects with a two-second total budget, using a client mode that never spawns a daemon. All failures return exit 0 with empty output. Errors are written to `$TALA_HOME/sessions/<connector>/hook.log` (size-capped) for diagnosis, never to stdout or stderr, which harnesses may show to the model.

### OpenCode plugin shells out to the same runner
The plugin is a small generated TypeScript file that calls `<tala-path> connector hook opencode <event>` with a JSON payload `{session_id, cwd, source}`, then applies the text: appended to `output.output` in `tool.execute.after`, and through whichever session-start and compaction mechanism task 1 confirms. If no reliable mechanism exists for session start in OpenCode, the connector declares that event unsupported and the existing `.tala/AGENTS.md` instructions cover registration, as they do today.

## Risks / Trade-offs

- [Harness hook formats change] → Connectors are versioned data; `check` flags drift; each connector has an integration test that replays a recorded payload and asserts the rendered output.
- [OpenCode tool-output mutation does not reach the model] → Task 1 tests this with a marker string before the connector is built; the fallback is a `noReply` session prompt, and if neither works the OpenCode connector ships with compaction and start reminders only and the gap is documented.
- [Context noise from repeated notifications] → Change-only reporting with a 15-minute repeat; output bounded by the attention summary's three-request limit.
- [Per-tool-call cost] → One loopback request; `tala status` returns in under 10 ms on the development machine. Codex hooks can use `async: true` so they never delay a tool call, at the cost of context arriving at the next safe point.
- [Session identifier unavailable or reused] → If the payload has no session key, the runner is silent. A harness that reuses identifiers across `clear` would carry the identity forward; this is acceptable because a cleared session in the same harness window is the same agent from the user's point of view.
- [Writing to user-level harness configuration] → Opt-in command, dry run, exact ownership markers, rollback on failure, and remove restores the prior state.

## Migration Plan

Additive. No existing configuration is changed until a user runs `tala connector install`. Rollback is `tala connector remove <name>` at each scope used, or downgrading the binary, after which `check` from a newer binary would report stale entries and the old binary ignores them (the hooks then fail silently on an unknown subcommand, which harnesses treat as a non-blocking hook error). Remove before downgrading.

## Open Questions

- Whether the Codex desktop app runs project-scope `.codex/hooks.json` for trusted projects in the same way as the CLI. Verified in task 4; it affects only the documented scope advice.
