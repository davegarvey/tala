# Design

## Context

See proposal.md for motivation. Relevant current state (protocol 2, 0.34.0):

- `register` creates an `agt_` ID and a daemon-side token. The CLI stores the credential at `TALA_HOME/identities/<agt_id>.json` and loads it from the public ID given to `--agent` (`client::credential`). Knowing a public ID is therefore enough to act as that agent.
- Sends without `--key` use a random UUID key (`cli.rs`), so retries are only idempotent when the caller supplied a key. The daemon enforces `UNIQUE(sender,key)` permanently.
- `client::call` maps every transport or decode failure to `CONNECTION_ERROR`. `client::ensure` treats any failed liveness probe as a dead daemon and spawns a new one.
- `wait_inbox` polls with 5-second requests; an error from any poll propagates and exits 1, even after `send --wait` has stored its request.
- The project key is `git rev-parse --show-toplevel`, which differs per linked worktree.
- Output is text unless `--json`; errors go to stderr as an envelope under `--json`.
- `init` writes `.tala/AGENTS.md` and, when `.opencode/` exists, an OpenCode skill and command.

Constraints: the CLI↔daemon protocol is versioned and must be bumped on incompatible change; identity is never derived from directory; reads other than inbox consumption never acknowledge.

## Goals / Non-Goals

**Goals:**
- Every behaviour works through the CLI and generated instructions alone, identically in every harness.
- An agent can always decide its next action from the JSON result without parsing prose.
- An agent learns of waiting work whenever it uses Tala, without a separate polling habit.

**Non-Goals:**
- Harness hooks (session-start registration, prompt injection, stop blocking). Each harness has a different, partly experimental hook system with its own configuration scope and trust model, and OpenCode cannot block a stop. Hooks would be proposed separately, starting with one harness, if hookless evaluations show agents missing messages or losing their handle.
- Webhooks or command notifications on delivery. A coding agent has no listener to receive them; they suit orchestrators and human notification, and would be proposed when such a consumer exists.
- Push delivery into running harness sessions, such as Claude Code channels or OpenCode's server API. Tracked as a possible future transport, not built now.
- Defending against a hostile process running as the same OS user. Such a process can read `TALA_HOME` directly. Handles prevent accidental cross-use between cooperating agents, not deliberate theft.
- A daemon-side session lease enforcing one process per identity. Handles remove the main path to sharing; a lease can follow if evaluations show sharing still occurs.
- Merging commands (for example `send`/`reply`/`post`) into fewer verbs. Deferred until evaluations show agents choosing the wrong command.
- File-editing claims or locks between agents.

## Decisions

### 1. Handles are a CLI-side secret; the wire credential is unchanged

`register` generates a handle `th_<32 hex>` and stores the credential at `TALA_HOME/handles/<sha256(handle)>.json` with mode 0600. `--agent`/`TALA_AGENT` accept only `th_` values; an `agt_` value fails with `IDENTITY_ERROR` explaining the difference. The daemon continues to authenticate `{agent, token}`.

Alternative considered: make the daemon token itself the handle. Rejected because it would change the wire credential and put the bearer token in agent context and shell history; hashing a separate handle keeps the token off disk-visible names and allows reissue without rotating the daemon token.

Reissue moves `identities/<agt_id>.json` to the hashed handle path and deletes the original, so it can succeed only once.

### 2. Content-derived keys with a daemon-side window

Without `--key`, the CLI sends `auto:<sha256(sender, operation, target, request flag, canonical parts)>`. For `auto:` keys the daemon looks up the newest message from that sender with that key prefix: inside the window it returns the original with `duplicate: true`; outside it stores a new message with the key suffixed by the new message ID, preserving `UNIQUE(sender,key)`.

Alternative considered: a time bucket in the CLI hash. Rejected because two retries straddling a bucket boundary would not deduplicate. The window is applied by the daemon clock, so it is consistent across processes.

### 3. Error contract lives on `Failure`

`Failure` gains `retry`, `stored` and `next`. The daemon sets them for application errors; the CLI sets them for transport errors and for wait failures after storage. `main` maps `stored != null` from a wait to exit 4. Help output gains an error-code table generated from one list in code, so the documentation cannot drift.

### 4. Transport classification

Liveness becomes a three-way result: live, refused (`io::ErrorKind::ConnectionRefused`), or unreachable (any other connect error or timeout). Only refused, or absent metadata, leads to spawning. Unreachable fails with `DAEMON_UNREACHABLE`, carrying the OS error. Transport codes are `CONNECTION_ERROR`, `DAEMON_UNREACHABLE`, `DAEMON_START_FAILED` and `PROTOCOL_MISMATCH`; application codes come only from a decoded daemon envelope.

### 5. Resilient wait

Inside `wait_inbox`, transport errors are retried with capped backoff (200 ms to 2 s) until the deadline. A permanent failure in `send --wait` returns the stored ID and `next: "tala inbox --wait --reply-to=<id>"`. `inbox` gains a public `--reply-to` flag; the `Inbox` operation already supports it.

### 6. JSON by default

`--human` selects the existing renderer; `--json` is retained as an accepted no-op for compatibility. Errors go to stderr as an envelope in both modes, with text form only under `--human`.

### 7. Attention: one query, two views

A new `Attention` operation returns incoming requests, unread messages (first page, peek semantics), outgoing requests and stale counts in one transaction, so the summary is consistent and costs one round trip. Bare `tala` calls it directly.

Ambient attention reuses the same query in compact form. The daemon runs it at the end of every authenticated operation's transaction, after the operation's own effects, and returns it as an optional `attention` field on the success envelope. Placing it on the envelope rather than in `data` keeps every command's `data` schema unchanged and lets the renderer print it uniformly under `--human`. Running it inside the operation's transaction means an `inbox` that consumes the last message reports zero unread in the same response.

Alternatives considered: computing attention in the CLI with a second request. Rejected because it doubles round trips and can report state that differs from the operation's result.

`age_seconds` is computed by the daemon at response time for every returned message. Staleness is computed, never stored, against a fixed 24-hour threshold; no flag tunes it, and evaluation results would change the constant.

`--json` and `register --reissue` are transitional: both work but are hidden from help, and are removed in a later release once protocol 2 homes are gone.

### 8. Repository-scoped project key

The project is the parent of `git rev-parse --path-format=absolute --git-common-dir`, canonicalised; outside git it remains the directory. `Context` gains `worktree` (the show-toplevel path). Existing registrations keep their stored project value; only new registrations use the new key.

### 9. Protocol 3

The new operation, the envelope `attention` field, new `Failure` fields and auto-key semantics are incompatible with protocol 2 daemons. `PROTOCOL_VERSION` becomes 3; the existing mismatch error applies. SQLite schema changes are limited to indexes supporting the automatic-key lookup and the attention query; staleness is computed.

## Risks / Trade-offs

- [Breaking `--agent` semantics strands running agents] → `IDENTITY_ERROR` names `register --reissue`; the agent guide and `init --refresh` explain the upgrade; release notes flag it.
- [Agents lose the handle on context compaction, with no hook to re-inject it] → the guide instructs agents to record the handle in their working notes; the shared-checkout evaluation measures how often it is lost, and that result decides whether hooks are proposed.
- [Agents not using Tala for a while miss waiting work] → ambient attention covers active use; the guide tells agents to run `tala` at work boundaries and, where the harness supports background commands, to wait in the background. Evaluations measure the remaining gap.
- [Ambient attention adds a query to every operation] → it reads indexed columns for one agent and returns counts plus at most three IDs. Measured in a release build on a home with 10,000 messages: about 40 µs per call for an agent with three open requests (against 4 µs for the same call without attention), and about 6 ms in the extreme case of 5,000 open requests to one agent. Both are small beside process start and the loopback request (`store::tests::attention_cost`).
- [Content-derived keys suppress a deliberate identical message within the window] → `duplicate: true` is explicit in the result, and `--key` with a new value forces a new message.
- [Treating timeouts as unreachable could hide a dead daemon] → the error preserves the OS reason and its `next` command is `tala status`, which distinguishes the cases without starting a daemon.
- [Changing the project key splits boards for existing worktree registrations] → existing agents keep their stored project; new registrations converge, and `board --project` reaches old boards explicitly.

## Migration Plan

1. Stop any protocol 2 daemon with its own binary (`tala stop`), as for previous protocol bumps.
2. Install the new release; the daemon starts on first use and adds its indexes.
3. Existing agents run `tala register --reissue=<agt_id>` once and use the returned handle, or register afresh.
4. Run `tala init --refresh` per project to update the generated instructions.

Rollback: stop the protocol 3 daemon and reinstall the previous release. Messages remain in SQLite; handles are unused by the old CLI, and reissued identities need their credential restored from the handle file, which the release notes document.

## Open Questions

- Default deduplication window and stale threshold, to be tuned from evaluation results.
