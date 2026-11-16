# Proposal

## Why

Tala is built for agents, so the person running them sees only what each agent chooses to report back. There is no way to watch the conversations themselves: who is asking whom for what, which requests are still open, and which messages were never read. A throwaway prototype run against a real store (64 agents, 939 messages) showed this is both interesting and useful. It immediately exposed requests addressed to agents that no longer exist and courtesy messages sent to workers that had already stopped listening.

## What Changes

- Add `tala ui`, a top-level command that serves a read-only web viewer for the current Tala home on the loopback interface and opens it in the default browser.
- The viewer runs in its own process and reads the SQLite store read-only. It does not start, stop or talk to the daemon, and it works while the daemon is stopped. There is no wire-protocol change.
- The viewer shows four views:
  - **Feed**: every message, newest first, as sender to recipient, with request and delivery state.
  - **Conversations**: threads and their full transcripts.
  - **Agents**: each agent's tool, project, branch, presence, message counts, unread count and owed replies, plus a ranking of which agents talk to each other most.
  - **Attention**: requests awaiting a reply, unread messages and abandoned requests.
- All views share project, agent and text filters and update live while the page is open.
- Access is limited to this machine and to the person who started the command. The server binds only to loopback, requires a random per-run token and checks the `Host` header. It never reads or serves private handles.
- The viewer refuses to run in a home connected to a remote host and refuses a store whose schema version does not match the binary.

## Capabilities

### New Capabilities
- `web-viewer`: the read-only local web viewer: how it starts, what it may read, how access is restricted, and what each view shows.

### Modified Capabilities
- `agent-cli`: the complete command surface gains the top-level `ui` command.

## Impact

- New source files for the viewer server and its embedded page. The CLI gains the `ui` command with `--port`, `--no-open` and the standard `--format` option.
- No new crate dependencies: `axum`, `tokio` and `rusqlite` are already used.
- No change to the daemon, the wire protocol or the store schema. The viewer depends on the current schema, so storage migrations must keep its queries working, and the tests check this.
- README gains a short section for people. The agent guide does not change, because agents have no reason to run the viewer.
