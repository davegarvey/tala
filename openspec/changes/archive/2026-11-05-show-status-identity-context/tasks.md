# Tasks

## 1. Status context and rendering

- [x] 1.1 Add safe selector-source and status-context data, using existing no-start RPCs; verify the CLI compiles and `tala status` still works without a daemon.
- [x] 1.2 Add the human status renderer with escaped names/paths; verify JSON and human output report the same context fields without private selector material.

## 2. Regression coverage and specification

- [x] 2.1 Add tests for each selector source, absent/invalid selectors, stopped/stale/incompatible daemon context, and no-start behavior; verify no inbox or board receipts change.
- [x] 2.2 Update and archive the OpenSpec change; verify `openspec validate --all` passes.
- [x] 2.3 Run formatting, clippy, and the full test suite; verify the change is ready for PR.
