# Proposal

## Why

When an agent changes directories after registering, Tala continues to attach only its registered checkout context to messages. In a real lookup task, a peer trusted that context, searched the wrong checkout, and needed an extra clarification before finding the file.

## What Changes

- Preserve registered agent identity and project-board membership while adding the canonical invocation working directory to outgoing message context.
- Document which context fields describe registration and which describe the current command.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-messaging`: Direct messages and board posts expose the command's current working directory separately from registered identity context.

## Impact

Context serialization in `src/models.rs`, message construction in `src/cli.rs`, agent guide, and messaging specification. The field is optional for compatibility with existing registrations and stored messages.
