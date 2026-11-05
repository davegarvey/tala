## 1. Store recipient observation
- [x] 1.1 Migrate the messages table to retain the direct recipient listening observation.
- [x] 1.2 Include the saved point-in-time observation in new direct send and direct reply results and idempotent key replays.
- [x] 1.3 Cover active listening, expired lease, inactive recipient, direct reply routing, and duplicate replay in store tests.

## 2. Human output and specification
- [x] 2.1 Render the observation as advisory, and state that an unlistened message remains queued.
- [x] 2.2 Add and sync the agent-messaging spec delta, then archive this change.
- [x] 2.3 Run formatting, full tests, clippy, and OpenSpec validation.
