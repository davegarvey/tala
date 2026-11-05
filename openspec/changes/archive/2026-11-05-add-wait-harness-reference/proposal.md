# Proposal

## Why

The listening guide explains how to use `tala wait`, but harness authors still have to infer how notification, non-consuming reads, acknowledgement, and cursor persistence fit together. A tested reference loop will reduce lost work and accidental duplicate processing when a supervisor restarts.

## What Changes

- Add a copyable Bash supervisor loop to the agent guide that uses a selector file, separate durable inbox and board cursors, and an application-provided message handler.
- Explain bounded inbox acknowledgement, at-least-once processing, duplicate IDs across inbox and board, exit handling, and restart behavior.
- Add focused guide assertions and an isolated end-to-end test for bounded acknowledgement when a new message arrives during processing.

## Capabilities

This is a documentation and test change. No system behavior or capability requirements change; the change opts out of spec deltas with `skip_specs: true`.

## Impact

- `docs/agent-guide.md` and its bundled `tala guide listening` output.
- Guide assertions and end-to-end tests.
