# Proposal

## Why

The long-lived Tala home accumulates projects and agent registrations, so a fresh agent can mistake old work for current work. Tala already retains useful history, but it has no project-level lifecycle or concise overview to separate work that is active, inactive, or explicitly finished.

## What Changes

- Add first-class project lifecycle records tied to Tala's existing canonical repository identity.
- Let the first agent registered for a project own it and explicitly close or reopen it.
- Derive a binary stale status from 30 days without project activity; it is not user-settable and clears when activity resumes.
- Add `tala list-projects`, showing open, non-stale projects by default, with clear options to include stale and closed projects.
- Preserve historical agents, messages, boards, and threads when a project is stale or closed.

## Capabilities

### New Capabilities
- `project-lifecycle`: Project ownership, close/reopen lifecycle, derived staleness, and server-wide project discovery.

### Modified Capabilities
- `agent-cli`: Add the project list and owner lifecycle commands, including default visibility and overrides.

## Impact

The change adds a project table and migration in SQLite, project lifecycle operations to the CLI/daemon protocol, and project-list rendering and help. Existing projects will be initialized from registered project contexts during migration; the earliest registered agent for each canonical project becomes its owner. Project age will use project activity tracked from registration and messages, with the latest known agent activity as the migration seed for existing records. Existing history and message behavior remain available regardless of project status.
