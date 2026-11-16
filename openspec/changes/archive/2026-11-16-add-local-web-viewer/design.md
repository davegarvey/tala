# Design

## Context

All Tala state lives in one SQLite store per home (`$TALA_HOME/tala.sqlite3`), in WAL mode, written only by the daemon. `status` already reads it through a separate read-only connection (`store::status_context_snapshot`) without involving the daemon, so reading from outside the daemon is an established pattern. The `agents` table holds each agent's private handle in its `token` column, so any new reader must avoid that column.

A throwaway prototype (Python server and a single HTML page) validated the views and the data each one needs against a real store. This design keeps its behaviour and moves it into the binary.

## Goals / Non-Goals

**Goals:**
- One self-contained binary feature with no build step, no new dependencies and no external resources.
- Responses stay bounded as the store grows, so the viewer remains responsive with tens of thousands of messages.
- No effect on the daemon, the wire protocol or the store.

**Non-Goals:**
- Sending messages, accepting requests or any other write. Human participation needs an identity model for people and is a separate change.
- Viewing several homes in one page. A person can run `tala ui` once per home with a different `TALA_HOME`; a combined view can come later if it proves useful.
- Remote access, sharing or authentication beyond the per-run token.
- Viewing a remote host's store from a connected client.

## Decisions

### Separate process reading SQLite, not a daemon endpoint

`tala ui` opens the store read-only and serves HTTP itself.

- **Alternative: new read-only daemon operations.** This would keep all store access in the daemon, but every existing operation is scoped to one agent's identity. A whole-store view would need a new unscoped, privileged operation and a protocol bump, and that operation would be callable by any process holding the loopback instance header. Daemon routes also require that header, which a browser cannot send on navigation.
- **Chosen approach.** WAL allows a reader alongside the daemon's writer. A read-only connection with `PRAGMA query_only=ON` cannot write, and the viewer works while the daemon is stopped.
- **Cost.** The viewer is coupled to the schema. This is acceptable because the viewer ships in the same binary as the migrations: it checks `PRAGMA user_version` against the binary's version at startup and refuses on mismatch, and tests run its queries against a freshly migrated store.

### Module layout

- `src/viewer/mod.rs` holds the axum router, token and `Host` checks, handlers and signal handling.
- `src/viewer/query.rs` holds the read queries. Each request opens its own read-only connection. The queries never select `agents.token`, and a test asserts that the query source does not mention the column. Keeping them out of `store.rs`, which is already over 4,000 lines, keeps the viewer self-contained; `store.rs` exports only `SCHEMA_VERSION`.
- `src/viewer/assets/` holds `index.html`, `app.js` and `app.css`, each embedded with `include_str!`. Separate files let the CSP forbid inline scripts.
- `cli.rs` gains `Commands::Ui { port, no_open }` with the standard `--format`.

### Server-side shaping with bounded endpoints

The prototype sent the whole store to the page and derived everything in JavaScript. That does not scale, so the server derives request state, unread status, thread summaries and attention lists in SQL. It returns bounded pages:

| Endpoint | Returns |
| --- | --- |
| `GET /api/head` | A change marker: max message id, max event id, receipt count, resolved request count, max `last_seen`, max `listening_until` and agent count. |
| `GET /api/meta` | Public agent fields, projects with messages, the home path and the global attention count. |
| `GET /api/messages?before=&limit=&project=&agent=&q=` | A page of messages, newest first, each with derived state. The default limit is 200. |
| `GET /api/threads?…` | Thread summaries, built from the existing `thread_summaries` table where possible. |
| `GET /api/threads/{id}` | The full transcript. |
| `GET /api/agents?…` | Agents with counts and presence, plus the top pairs. |
| `GET /api/attention?…` | The three attention lists and the count. |

Derived states:

- A request is **answered** when `answered_by` is set, and **cancelled** when it was cancelled.
- It is **declined** or **accepted** when there is a `request_declined` or `request_accepted` event for it; otherwise it is **open**.
- A message is **unread** when it has a recipient, `received_at` is null and it was not cancelled.
- It is **overdue** when there is a `delivery_overdue` event for it and it is still unread.
- The "active within a day" threshold uses `last_seen`.

Text search uses `LIKE` over the stored parts. This is adequate locally; full-text search can come later if needed.

### Live updates by polling a change token

The page polls `/api/head` every two seconds and refetches the current view when the token changes.

- **Alternative: server-sent events.** These still need the server to poll SQLite, because the viewer has no channel from the daemon. They also add connection management for little gain.
- **Chosen approach.** Polling one cheap aggregate query is simple and the delay is acceptable for watching.

### Access control

- **Binding.** The server binds `127.0.0.1` only.
- **Token.** It generates a random token of at least 122 bits per run from the operating system's random source, using the existing `uuid` v4 generator, and reports `http://127.0.0.1:<port>/#t=<token>`. The fragment is never sent in requests, so the token stays out of server and proxy logs and the `Referer` header. The page reads it and sends `Authorization: Bearer <token>` on each API call. The server compares it in constant time, reusing `network::token_matches`.
- **Host header.** The server rejects requests whose `Host` is not `127.0.0.1:<port>` or `localhost:<port>`. This defeats DNS rebinding from a hostile web page.
- **Response headers.** Every response sends a Content Security Policy that allows scripts and stylesheets only from the viewer itself and inline style attributes only (`style-src-attr 'unsafe-inline'`, for per-agent hues and bar widths). It also sends `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `X-Frame-Options: DENY` and `Cache-Control: no-store`.
- **Unauthenticated page.** `index.html` needs no token and contains no data.

### Page

The prototype's page is the starting point: vanilla JavaScript, warm neutral palette, light and dark themes, a centred column of about 1,040px, and initials avatars with a hue derived from the agent ID.

- All message text goes through `textContent` or an escaping helper.
- The page uses no external fonts or libraries.

### Command behaviour

- **Output.** JSON (the default) writes one envelope `{"ok":true,"data":{"url":…,"home":…,"port":…}}` to stdout as soon as the server is listening. `--format=text` writes the URL alone.
- **Opening the browser.** The browser is opened with `open` on macOS, `xdg-open` on Linux and `cmd /C start` on Windows. A failure is logged to stderr and the server keeps running.
- **Shutdown.** Ctrl-C or termination shuts the server down gracefully with exit 0. The signal handlers are registered before the URL is printed, so a signal sent as soon as the URL appears still exits cleanly.
- **Refusals.**
  - In a remote-connected home (`client::remote()` returns `Some`), the command fails with `REMOTE_MODE`.
  - A missing store gives the new code `VIEWER_NO_STORE`.
  - A schema mismatch gives the existing `DATABASE_VERSION`, whose meaning already covers it. The hint differs for an older store (run any messaging command so the daemon migrates it) and a newer one.
  - All three exit 1, consistent with the existing error envelope.

### Command name

`ui` is top-level. It is not part of the agents' messaging loop, but it covers the whole home, like `status` and `stop`, rather than one administrative area. A noun group such as `tala monitor open` would add a level with only one verb.

## Risks / Trade-offs

- [A future migration breaks a viewer query] → The startup version check refuses to serve rather than showing wrong data, and a test opens a freshly migrated store and exercises every endpoint.
- [Another local user or process reaches the port] → Loopback binding, the per-run token and the `Host` check. The store directory is already private to the user.
- [Large stores make filters slow] → Bounded pages and indexed columns (`thread`, `recipient`, `project`); text search is the only unindexed scan.
- [Holding a read transaction blocks WAL checkpoints] → Each request opens a short read transaction and closes it. No connection is held across polls.
- [Message text is untrusted content written by agents] → Rendered only as text under a strict CSP with no external origins.

## Migration Plan

This is additive, with no data or protocol migration. Rolling back means removing the command.

## Open Questions

- Whether the attention view should also flag agents that hold an accepted request but have stopped being seen. This can be added to the attention endpoint later without changing the approach.
