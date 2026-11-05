# Design

## Context

`identity_context` loads the registered agent context so project identity and board membership remain stable. It currently refreshes only `working_directory`, leaving Git metadata stale after a command moves to another worktree, branch, or revision.

## Goals / Non-Goals

**Goals:** Put invocation-time Git metadata on every outgoing message while retaining registered project identity.

**Non-Goals:** Change agent selection, registration metadata, board routing, or the wire format.

## Decisions

At outgoing context construction, resolve `context(None)` for the process's current directory, copy its `worktree`, `branch`, `revision`, and canonical `working_directory` into the registered context, and leave the registered `project` untouched. `context` already resolves Git metadata with Git commands and returns absent Git fields outside a worktree. The explicit copy avoids replacing canonical project with the invocation directory's project, which would move board posts or identity context. This also handles nested directories because Git resolves the containing worktree while the canonical working directory remains the nested invocation path.

Refreshing all four invocation fields in the shared helper covers direct sends, replies, handoffs, and board posts. A helper used only for direct `send` would leave the same stale metadata on the other outgoing message forms.

## Risks / Trade-offs

- **The invocation directory may not be accessible to the receiver** → Keep the fields as local context hints; message content and routing do not depend on them.
- **Git commands may fail or return no branch in a detached checkout** → Preserve the existing optional-field behavior: report fields only when Git can resolve them, and do not fabricate values.

## Migration Plan

No data migration or protocol change is needed. Existing registered contexts remain readable; each new outgoing message computes its own current context.
