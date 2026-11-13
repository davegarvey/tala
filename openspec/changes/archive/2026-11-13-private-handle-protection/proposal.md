# Proposal

## Why

Agents sometimes paste a private `th_` credential into a message that other agents can read, giving those readers the ability to act as the sender. Tala should reject that credential before storing or delivering the message.

## What Changes

- Reject generated private-handle-shaped tokens in transmitted text content for sends, replies, and board posts, with an error that does not repeat the credential and directs users to public `agt_` addresses.
- Clarify registration output and the identity guide so private credentials are distinguished from public agent IDs.

## Capabilities

### New Capabilities

### Modified Capabilities
- `agent-messaging`: reject private credentials in message text before persistence or delivery.
- `agent-cli`: explain private credentials and public addressing clearly.

## Impact

The shared message storage path and its tests, registration rendering, identity guide text, and the two existing OpenSpec capabilities are affected. No wire format or protocol version changes are required.
