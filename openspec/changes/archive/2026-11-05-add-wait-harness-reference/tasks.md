# Tasks

## 1. Reference loop

- [x] 1.1 Add the durable wait/peek/process/ack/resume recipe and explain single-reader, dedupe, exit, and cursor rules; verify the Listening section contains the required command sequence and never evaluates `next.*` strings.
- [x] 1.2 Add focused isolated end-to-end coverage showing a message arriving during processing remains unread after bounded acknowledgement; verify it is fetched on the next pass.

## 2. Integration

- [x] 2.1 Run formatting, clippy, unit/e2e tests, and `openspec validate --all`; verify all succeed.
