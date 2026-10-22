# Proposal

## Why

When `send --wait` times out, Tala has already stored the request, but its timeout result does not say how to resume waiting. Agents must remember or rediscover the correlated inbox syntax, creating avoidable risk of resending work.

## What Changes

- Include an exact recovery command in timed-out `send --wait` results, targeting the stored request and preserving the requested timeout.
- Clarify that timeout means the request remains stored and must not be resent.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-cli`: A stored request timeout returns a direct recovery action.

## Impact

The send wait result in `src/cli.rs`, CLI tests, and agent CLI specification. No wire protocol or storage changes.
