# Sender read receipts: per-(session, sender) read state (B021)

## Why

Backlog B021 (promoted P2→P1 in cycle-12, reconfirmed fresh on main caf8718):
the sender of a message has **no way to tell whether the other agent picked it
up**. Repro (09:57Z):

```
beta: tala history sess_dc6nt      # beta reads alpha's question (msg 1)
alpha: tala list                    # → sess_dc6nt  cyc12-q  open  1 msgs *
alpha: tala list --json             # → no read metadata at all
```

After beta read the question, alpha's `list` is byte-identical to before the
read. There is no "delivered/read" signal anywhere in the CLI or API surface —
the core two-agent loop (send question → wait → reply) is a black box between
"sent" and "answered". This feeds missing-feature M4 (per-message delivery/read
state).

Root cause: the daemon has **no read-state concept**. Its read endpoints
(`/recap`, `/messages`, `/wait`) return messages but never record who has read
them; the CLI even sends `&sender=<name>` on `wait` (cli.rs wait path) but the
daemon's `WaitParams` doesn't parse it, so the identity is silently dropped.
Client-side cursors (`.tala/cursor`) are global-cursor-broken (B014, unmerged
#46) and cannot answer "did BETA read MY message?" anyway — read state must be
tracked daemon-side, per session, per sender.

## What Changes

- **Store (src/store.rs)**: add in-memory read state
  `HashMap<(session_id, sender), last_read_msg_id>`:
  - `record_read(session_id, sender, up_to)` — monotonically advances the
    sender's last-read id for the session.
  - `list_sessions` attaches a per-session `read_by: {sender: last_read_id}`
    map to each `SessionSummary`.
  - Read state is daemon-side and in-memory (transient across restarts, same as
    today's message store until #47 lands).
- **API (src/api.rs + src/models.rs)**:
  - `GetMessagesParams`, `WaitParams`, `RecapQuery` gain `sender: Option<String>`
    (the CLI already sends `&sender=` on wait; recap/check will now send it too).
  - Read endpoints record a read when messages are actually returned to a named
    sender: `recap_session` (history), `get_messages` (check), `wait_for_message`
    (existing + live-delivered paths), `wait_new_session`/`wait_all` (delivered
    session).
  - `SessionSummary` gains `read_by: HashMap<String, u64>` (serde default/skip
    when empty — old daemons/CLIs stay compatible).
- **CLI (src/cli.rs)**:
  - `history` (cmd_recap) and `check` (cmd_whatsup) append `&sender=<identity>`
    to their requests (identity = `.tala/config.json` name, falling back to the
    default sender — same resolution `wait`/`wait-new` already use).
  - `tala list` text: for each session, append `read: <agent>@<id>` for readers
    OTHER than the local identity (self-reads are noise in text).
  - `tala list --json`: `read_by` flows through `SessionSummary` unchanged
    (full map, includes self — machine-consumable).

## Capabilities

- Sender can answer "did beta pick up my question?" with one `tala list`:
  `sess_dc6nt  cyc12-q  open  1 msgs *  read: beta@1`.
- Automation can query read state via `list --json` → `read_by`.
- Read state is recorded per reading agent, so a session has one entry per
  reader (the highest message id that reader has seen).

## Impact

- **No base dependency**: read state is tracked daemon-side and does NOT depend
  on the unmerged per-session-cursor PR #46 (client cursors remain broken on
  main; this feature is orthogonal — it uses the server, not `.tala/cursor`).
- Backward compatible: `read_by` is omitted when empty; `sender` params are
  optional; old CLI responses still parse (`serde(default)`).
- Honest scope: read state is not persisted across daemon restarts (messages
  themselves aren't either until #47 merges). Receipts are "as of the current
  daemon lifetime".
- B004 (sender impersonation) is a design call; `--sender` spoofing also spoofs
  receipts — flagged, not fixed here.
