# Design

## Context

Thread access is checked in several store operations. Some paths already use `THREAD_NOT_VISIBLE`, while others classify a visibility failure as `INVALID_OPERATION`. The visibility query intentionally hides threads outside the acting identity's participant or project scope.

## Goals / Non-Goals

**Goals:**
- Make all visibility failures share one stable, non-disclosing error shape.
- Point users to identity-selection guidance and a safe thread-sharing path.

**Non-Goals:**
- Reveal which identity or project owns a hidden thread.
- Change thread visibility rules or grant access automatically.
- Change the wire or storage schema.

## Decisions

- Centralize the code, message, hint, and next command in a store helper and use it for every failed `visible_thread` check. The existing generic message, “Thread is missing or not visible to this agent,” remains because distinguishing those cases could reveal thread existence.
- The hint says the thread may be outside the current identity/project context, recommends selecting the original private identity or asking its owner to use `tala handoff`, and does not include any private handle or foreign project detail.
- Set `next` to `tala guide identity`, which is available without registration and explains selector-based identity selection. This gives the structured error one exact useful command while the hint handles the multiple recovery choices.

## Risks / Trade-offs

- A generic message cannot tell the caller whether the ID is mistyped or belongs elsewhere. This is intentional to preserve the existing privacy boundary; the hint and `next` command provide safe recovery.
- A handoff can only help when the owner can be identified through an existing conversation. The error will not expose that identity.
