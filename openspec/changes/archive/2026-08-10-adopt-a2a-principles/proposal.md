## Why

tala reinvents parts of the A2A protocol informally. Three A2A principles transfer directly to tala's loopback design without overextending it: structured message content (Parts instead of a single markdown string), idempotent sends (a client-supplied key so retries never double-post), and explicit CLI/daemon version negotiation (so a stale daemon doesn't silently break a new CLI).

## What Changes

- **Structured message content (Parts)**: `Message` content becomes a list of typed parts (`text`, `file`, `data`) instead of a single `String`. Text parts keep existing markdown semantics; file parts reference a path (for diffs/patches/code review); data parts carry a JSON blob. Message surfaces render the text part as today and annotate file/data parts. **BREAKING**: the wire format and persisted `messages.json` shape change; old messages load with a single text part.
- **Idempotent sends**: `tala send` generates a random idempotency key once per invocation and reuses it across retries; the daemon rejects a duplicate key for the same sender+session+key triple. **BREAKING**: the send request payload gains a required idempotency key.
- **CLI/daemon version negotiation**: `DaemonInfo` and `daemon.json` gain a protocol version; the CLI verifies compatibility before issuing commands and errors clearly on mismatch.

## Capabilities

### New Capabilities
- `message-parts`: structured message content — typed parts replace the flat string, with rendering and snippet rules across surfaces.
- `send-idempotency`: client-generated idempotency keys, daemon-side dedup, and retry-safe `tala send` semantics.
- `daemon-compat`: daemon protocol version advertisement and CLI compatibility checking.

### Modified Capabilities
- `message-intent`: the Message model and content rendering change — surfaces render parts, and pending-view snippets are drawn from the text part.

## Impact

- `src/models.rs`: `Message` content → parts; `DaemonInfo` gains version; send request gains idempotency key.
- `src/store.rs`: persistence format for `messages.json`; dedup check on add.
- `src/api.rs`: send handler validates idempotency keys and stores parts.
- `src/cli.rs`: `send` key generation + retry reuse; daemon compatibility check in `ensure_daemon_running`.
- `src/daemon.rs`: version field written to `daemon.json`.
- `tests/e2e.rs`: fixture updates for the new wire format plus new dedup and version-mismatch tests.
