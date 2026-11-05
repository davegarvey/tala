# Tasks

## 1. Startup lock generation

- [x] 1.1 Route startup attempts and stop barriers through `startup-v2.lock`; verify every current coordination path uses the same filename and `daemon.lock` remains unchanged.
- [x] 1.2 Add an end-to-end regression holding legacy `startup.lock` while a current command starts the daemon; verify the command succeeds and only one daemon owns the home.
- [x] 1.3 Update lifecycle coordination comments and durable-daemon documentation; verify the docs name the legacy-to-current lock migration accurately.

## 2. Integration and delivery

- [x] 2.1 Run formatting, tests, clippy, and strict OpenSpec validation; verify all pass.
- [x] 2.2 Sync the durable-daemon delta into the main spec; verify the legacy-lock scenario is present and `openspec validate --specs` passes.

## Workflow follow-up

- Archive the change after specs are synced and the implementation tasks are complete.
- Open a PR, merge after required checks pass, then build and install the merged release on this machine.
