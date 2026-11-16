# Tasks

## 1. Read-only store access

- [x] 1.1 Add a read-only opener in `src/viewer/query.rs` that opens the store with `SQLITE_OPEN_READ_ONLY` and `PRAGMA query_only=ON`, reads `PRAGMA user_version`, and reports a mismatch with the binary's schema version. Verify with unit tests for a matching store, an older store and a missing file.
- [x] 1.2 Add the change-token query (max message id, max event id, receipt count, max `last_seen`, max `listening_until`). Verify with a unit test that the token changes after a send, a receipt and an event.
- [x] 1.3 Add the paged message query, newest first, with `before`, `limit` (default 200, bounded), `project`, `agent` and text filters, and derived request, unread and overdue states. Verify with unit tests for each derived state, for each filter and that consecutive pages neither overlap nor skip messages.
- [x] 1.4 Add the thread-list and thread-transcript queries. Verify with unit tests for participants, message counts, the awaiting-reply flag and transcript order.
- [x] 1.5 Add the agents query (tool, project, branch, presence, sent, received, unread, owed replies) and the pair ranking. Verify with a unit test using one coordinator and several workers.
- [x] 1.6 Add the attention query (awaiting reply, unread, abandoned, and the count) with the one-day activity threshold. Verify with unit tests for an active recipient and for a recipient last seen two days earlier.
- [x] 1.7 Ensure no viewer query reads `agents.token`. Verify with a test that scans the viewer SQL for the column and with a test that every endpoint's JSON for a seeded store contains no stored handle value.

## 2. Viewer server

- [x] 2.1 Create `src/viewer/mod.rs` with an axum router bound to `127.0.0.1` on `--port` or an operating-system-assigned port, and a per-run token from `uuid` v4. Verify that a test server reports a loopback address.
- [x] 2.2 Add the `Host` header check and the bearer-token check, using `network::token_matches`, on every `/api/*` route, and the security response headers on every response. Verify with tests for a missing token, a wrong token, a token from another run, a foreign `Host` header and the presence of the headers.
- [x] 2.3 Add handlers for `/api/head`, `/api/messages`, `/api/threads`, `/api/threads/{id}`, `/api/agents` and `/api/attention` that each open a short read transaction per request. Verify with handler tests against a seeded store, including a run while a writer connection is committing.
- [x] 2.4 Verify that the store is unchanged after exercising every endpoint by comparing its schema version and row counts before and after in a test.

## 3. Page

- [x] 3.1 Port the prototype page to `src/viewer/assets/` (`index.html`, `app.js`, `app.css`): Feed, Conversations, Agents and Attention views, shared filters, the centred column, light and dark themes, and no external resources. Serve it with `include_str!` at `/`. Verify that the page loads with the network disabled and that a test asserts the HTML references no other origin.
- [x] 3.2 Switch the page from a whole-store snapshot to the paged endpoints, with "load older" in the feed and a bearer token read from the URL fragment. Verify in a browser against a store with more than one page of messages.
- [x] 3.3 Render all message text through escaping or `textContent`. Verify by seeding a message containing `<script>` and `<img onerror>` and confirming in a browser that it shows as literal text.
- [x] 3.4 Poll `/api/head` every two seconds, refetch the current view on change, and show a disconnected state on failure. Verify in a browser by sending a message while the page is open and then stopping the server.

## 4. Command

- [x] 4.1 Add `Commands::Ui { port, no_open }` to `src/cli.rs` with the standard `--format`. It writes the JSON envelope or a text URL to stdout once listening, logs to stderr, opens the browser unless `--no-open`, and exits 0 on Ctrl-C. Verify with an e2e test in `tests/e2e.rs` that starts `tala ui --no-open` in an isolated `TALA_HOME`, reads the URL, fetches `/api/head` with the token and sends an interrupt.
- [x] 4.2 Refuse with `VIEWER_NO_STORE`, `REMOTE_MODE` or `DATABASE_VERSION`, each with a hint and exit code 1. Verify with an e2e test for each.
- [x] 4.3 Confirm that `tala ui` neither starts nor contacts the daemon. Verify with an e2e test that runs it in a fresh home with a seeded store and asserts no daemon state file or process appears.
- [x] 4.4 Update the help-surface test so `ui` appears among the top-level commands. Verify with `cargo test`.
- [x] 4.5 Add a short "Watching your agents" section to `README.md` describing `tala ui`, its read-only nature and the per-run token. Verify that the documented commands run as written.

## 5. Integration checks

- [x] 5.1 Run `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` cleanly.
- [x] 5.2 Run the built binary read-only against the real store while its daemon is running, and confirm that every endpoint answers quickly and all four views, paging and the agent filter render without console errors. On an isolated home, confirm in a browser that markup in a message shows as literal text, that a reply sent while the page is open appears with its request marked answered, and that stopping the server shows the disconnected state.

## Workflow follow-up

- Sync the delta specs into `openspec/specs/` and sweep every main spec for statements that the change has made stale.
- Archive the change on the feature branch, in the same PR as the implementation.
