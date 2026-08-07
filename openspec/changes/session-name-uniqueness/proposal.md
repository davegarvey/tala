# Session name uniqueness (B017)

## Why

Backlog B017 (reconfirmed cycles 02–17, promoted P2→P1 in cycle-17):
duplicate session names are accepted **silently**. `session create --name dup17`
twice → both succeed. `session rename <id> dup17` → succeeds with **no
duplicate check** (cycle-17 live: three sessions named `dup17` in one
daemon). The failure lands later, at the worst moment: the first `use dup17`
errors `Multiple sessions named 'dup17'. Use session ID instead.` (exit 1),
and any name-addressed handshake (`use ops-handshake`, `send ops-handshake
"…"`) becomes a coin flip or a silent misroute.

PR #55 made names a first-class addressing scheme (`resolve_session_ref`).
Uniqueness is the missing guardrail: names are an addressing *key*, so the
store must enforce the invariant at write time — exactly like IDs do.

## What Changes

Daemon-side enforcement + CLI error plumbing. Session IDs are untouched.

- **`Store::session_name_exists(name)`** (src/store.rs): true if any session
  carries that name.
- **`create_session` handler** (src/api.rs): when the request carries a
  `name` that already exists → `409 CONFLICT` with
  `{"error": "A session named 'X' already exists"}`; no session is created.
- **`Store::rename_session`** (src/store.rs): returns
  `Err("A session named 'X' already exists")` when *another* session already
  holds the target name. Renaming a session to its **own** current name stays
  a no-op success (existing `test_rename_noop_same_name`). `--force` does
  not override the uniqueness invariant (it only governs renaming an
  already-named session). The api.rs rename handler already maps `Err` → 409.
- **CLI** (src/cli.rs):
  - `auto_create_session`: check the HTTP status; on 409 parse the
    `ErrorResponse` and `fail(json_output, msg, "SESSION_NAME_TAKEN")` —
    previously a non-201 response would fall into `resp.json::<CreateSessionResponse>()`
    and produce a cryptic serde error.
  - `cmd_session_rename`: the 409 arm emits code `SESSION_NAME_TAKEN`
    (replacing the dead `SESSION_ALREADY_NAMED` label; nothing referenced it).
  - In `--json` mode both paths emit the standard
    `{"error": "...", "code": "SESSION_NAME_TAKEN"}` on stderr, exit 1.

## Capabilities

- `session create --name <n>` fails fast with a clear message when `<n>` is
  taken (exit 1; JSON error in `--json`).
- `session rename <id> <n>` fails fast when `<n>` belongs to another session
  (exit 1; JSON error in `--json`).
- Unnamed sessions, `send`-auto-created sessions, and duplicate *IDs* are
  unaffected.

## Impact

- **API**: new 409 path on `/api/sessions` (create) — additive; the rename
  409 path already existed but was unreachable (dead `Err` arm).
- **CLI**: create-with-duplicate-name changes from silent success to a loud,
  specific error — the point of the change.
- **Compat**: `test_rename_noop_same_name` (rename to own name) and
  `test_rename_succeeds_without_force` must stay green; they define the
  self-exclusion and force semantics.
- No openspec/dependency on unmerged PRs — clean base vs main caf8718.
