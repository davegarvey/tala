## 1. Message parts model

- [x] 1.1 Add `Part` enum (text/file/data, internally-tagged serde) in `src/models.rs` with `#[serde(tag = "type")]` and snake_case kinds
- [x] 1.2 Replace `Message.content: String` with `parts: Vec<Part>`: custom deserializer (accept `parts`, else legacy `content` → single text part) and serializer emitting `parts` plus derived `content` (text parts newline-joined) for old-binary read compat
- [x] 1.3 Update `SendMessageRequest` to accept optional `parts` or legacy `content` (at least one required; normalize to parts in the handler)
- [x] 1.4 Update `send_message` (src/api.rs:137) to normalize input into parts and return them in `SendMessageResponse`
- [x] 1.5 Update `Store` (`AddMessageParams`, `add_message_with`, snippet computation at src/store.rs:575) to carry parts and derive snippets from the first text part (or `[file]`/`[data]` placeholder)
- [x] 1.6 Update all message surfaces in `src/cli.rs` (send, history, wait, stream, listen, check) to render parts: text as-is, `[file: path]` / `[data: <json>]` annotations, `parts` in `--json` output
- [x] 1.7 Update the `--match` filter in the daemon's observe/listen path (src/api.rs:1324, 1399) to match text parts only (no-text-part messages never match)
- [x] 1.8 Add `--part text:<v> | file:<path> | data:<json>` repeatable flags to `tala send` (split on first colon; `data:` must parse as JSON; unknown kind → usage error; `allow_hyphen_values`; `--part` counts as content for the send gate, bypasses stdin fallback, auto-creates a session; mixing with positional content, `--message-file`, or `--stdin` → usage error)
- [x] 1.9 Verify legacy `messages.json` (string content) loads and renders identically via the deserializer

## 2. Send idempotency

- [x] 2.1 Add `idempotency_key: Option<String>` to `Message` (persisted, `skip_serializing_if` none)
- [x] 2.2 Make `idempotency_key` required on `SendMessageRequest`; reject missing keys with an error and no message stored
- [x] 2.3 Build in-memory `HashMap<(sender, key), message_id>` dedup index in `Store` at load; on add: unknown key → record; known key + equal serialized parts → return original message without storing or broadcasting; known key + different parts → 400 naming the conflict
- [x] 2.4 Add `duplicate: bool` to `SendMessageResponse`; CLI prints "duplicate suppressed (msg N)" and exits 0 on dedup; `--json` mode reports `duplicate`, original message id, and session id as typed fields
- [x] 2.5 Add `uuid` (v4) dependency; `tala send` generates one key per invocation at arg-parse time and reuses it across every retry path
- [x] 2.6 Retry the send POST on connection errors only (up to two additional attempts) with the same key; do not retry on HTTP error responses

## 3. Daemon version negotiation

- [x] 3.1 Add `PROTOCOL_VERSION: u32 = 1` const; add `protocol_version` to `DaemonInfo`, `StatusResponse`, and `daemon.json` write/read (serde default so stale files read as 0)
- [x] 3.2 Add version check inside `ensure_daemon_running` (src/cli.rs:482) against the live `/api/status` version: mismatch → error naming both versions with `tala stop` / upgrade remedy, nonzero exit, JSON error document in `--json` mode, no state-mutating command issued
- [x] 3.3 Add `allow_mismatch` variant for read-only commands (`status`, `discover`, `agents`): warn and exit 0; wire it into `cmd_discover`/`probe_daemon` (src/cli.rs:2213-2268) which bypass `ensure_daemon_running`
- [x] 3.4 Include protocol version in `tala status` output (human and `--json`)

## 4. Tests and documentation

- [x] 4.1 e2e tests: parts send/render/`--json`, `--part` validation errors, legacy content send (with key) and load
- [x] 4.2 e2e tests: dedup on retry (single message stored), key conflict error, missing key rejection, JSON-mode duplicate fields
- [x] 4.3 e2e tests: stale daemon blocks commands, `status`/`discover` warn without failing, fresh spawn works
- [x] 4.4 Update existing unit and e2e tests (`models.rs` round-trip, `store.rs` unit tests, `tests/e2e.rs`) for the new wire format (parts in responses, required key)
- [x] 4.5 Document the `PROTOCOL_VERSION` bump policy (any incompatible wire change bumps it) in AGENTS.md or a code comment
- [x] 4.6 Run `cargo fmt --check`, clippy with `-D warnings`, and the full `cargo test` suite
