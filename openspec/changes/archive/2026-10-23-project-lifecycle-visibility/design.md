# Design

## Context

Projects are canonical repository paths derived during agent registration. Agents and messages are stored separately; there is no project table, owner, lifecycle state, or server-wide project list. Linked worktrees already share a canonical project identity.

## Goals / Non-Goals

**Goals:**
- Persist project owner, explicit closed state, and last meaningful activity.
- Derive staleness at read time and keep it orthogonal to open/closed state.
- Make the default server-wide overview concise while keeping old projects and history reachable.

**Non-Goals:**
- Delete or expire agent identities, messages, boards, or threads.
- Prevent direct messaging or history access when a project is closed.
- Support project deletion, ownership transfer, or a configurable staleness threshold in this change.

## Decisions

### Use the existing canonical repository path as project identity

The current context builder resolves linked worktrees to their main worktree root. Project lifecycle records use that same path, so existing board membership and project identity do not change. The first agent registered for a new project becomes its owner.

### Migrate the current database and seed ownership from registration order

Add a projects table keyed by canonical path, with owner agent ID, closed state and last-activity timestamp. During migration, group existing agent registrations by project, assign the earliest stored registration as owner, and seed activity from the newest of their last-seen timestamps and associated message timestamps. Newly registered projects are inserted with that agent as owner and current activity. This preserves existing identities and avoids a manual migration step.

### Track meaningful project activity and derive stale status

Registration, message creation and owner-initiated reopening update the relevant project's last-activity timestamp. Closing, listing, history reads, status checks and inbox peeks do not. At read time, a project is stale when its last activity is at least 30 days old. The threshold is a single documented product default; this change adds no per-user setting or manual stale command.

### Make close a reversible catalog state

`close-project` and `reopen-project` operate on the selected agent's registered project and require that agent to be the stored owner. Closing only changes lifecycle visibility: agents remain addressable, and existing direct and board history stays readable. New activity does not reopen a closed project. A stale status may change independently as activity arrives.

### Add a server-wide list with explicit inclusion options

`list-projects` returns project path, owner, lifecycle state and last activity. It defaults to open, non-stale projects, reports counts of omitted stale and closed projects, and gives exact commands to include them. `--stale` includes stale projects, `--closed` includes closed projects, and `--all` includes both categories. Closed projects are not silently folded into the stale count.

### Version the wire protocol and keep the storage change additive

Add list/close/reopen RPC operations and increment `PROTOCOL_VERSION` so an older daemon cannot silently accept a new operation. Add the project table and backfill transactionally without changing the existing SQLite schema version: the change is additive, and protocol 3 binaries can safely ignore the new table during rollback. The daemon's existing version-mismatch behavior remains the recovery path for a CLI/daemon version split.

## Risks / Trade-offs

- [The earliest stored registration may not be the human-meaningful project owner in an existing database] → Make the deterministic migration choice visible in `list-projects` and document the rule; ownership transfer remains outside this first version.
- [Thirty days may hide a project that is intentionally paused] → Report hidden counts and provide `--stale`/`--all` overrides; the stale status is reversible when activity resumes.
- [A closed project can still receive messages] → Render its closed status in project listings and keep closure explicitly scoped to lifecycle visibility, not message authorization.
- [Old agents may exist without an associated project record] → Backfill every distinct registered canonical project in the migration before accepting project-list operations.

## Migration Plan

1. On opening the current database, transactionally add and backfill project records from registered agents and their messages.
2. New registrations create missing project records and set the first registering agent as owner.
3. Deploy the CLI/daemon with the incremented protocol version; an older live daemon is rejected by the existing protocol compatibility check and must be restarted with the matching binary.
4. Rollback uses the previous binary only after stopping the new daemon. Its protocol 3 store ignores the additive project table; messages and existing agent data remain intact.
