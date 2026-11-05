# Proposal

## Why

When an agent uses a different identity or project context from the one that can see a thread, the current error gives too little recovery guidance. Related history, handoff, and message operations also use inconsistent error classifications, making the cause harder to diagnose.

## What Changes

- Use one non-disclosing `THREAD_NOT_VISIBLE` error for inaccessible or missing thread references across history, handoff, message status, board replies, and direct messages continuing a thread.
- Keep the error generic so callers cannot probe whether a thread exists outside their visible scope.
- Explain that the thread may belong to another identity or project context and point to safe recovery guidance without exposing private handles or foreign project metadata.

## Capabilities

### New Capabilities

### Modified Capabilities

- `agent-cli`: Actionable thread-visibility errors provide safe recovery guidance while preserving privacy.
- `agent-messaging`: Operations requiring a visible thread use a consistent non-disclosing visibility error.

## Impact

Affected areas include store-side authorization checks for history, handoff, message status, and board replies; the documented error catalog and identity guide; e2e coverage; and the agent-cli and agent-messaging OpenSpecs. No wire or storage schema change is expected.
