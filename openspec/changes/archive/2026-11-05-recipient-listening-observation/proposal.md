# Recipient listening observation

## Why

Direct send and reply results currently confirm the stored message without showing whether Tala has recently observed the recipient waiting for messages. Senders need to know when a message is queued for an agent that is not currently listening.

## What Changes

Add an advisory observation of the recipient's lease state when Tala accepts the message. It does not guarantee a response. Idempotent replays retain the original observation, just like they retain the original message ID.

- Add `recipient_presence: {"listening": boolean}` to direct sends and direct replies.
- Persist the observation with each direct message so message-key retries return the same value.
- Render a human-readable explanation and explicitly say the message remains queued when the recipient was not listening.
- Do not add presence to board posts or board replies, and do not change listener guidance.
