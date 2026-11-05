# Tasks

## 1. Lifecycle coordination

- [x] 1.1 Add atomic per-home stop generation/state and one-shot startup reservations; verify pre-publication stop and targetless-stop E2Es pass
- [x] 1.2 Make stop target a specific daemon instance and wait for both metadata removal and daemon lifetime lock release; verify active and repeated stop E2Es pass
- [x] 1.3 Preserve stale-marker recovery without recovering while the stop-operation lock is held; verify ordinary commands wait and waits exit during shutdown

## 2. Wait behavior and compatibility

- [x] 2.1 Bind `wait` and `inbox --wait` to a lifecycle generation and return `DAEMON_STOPPED` with exit 5; verify stop and event/receipt behavior tests pass
- [x] 2.2 Keep crash recovery and finite timeout semantics; verify #126 recovery and timeout exit 3 regressions pass
- [x] 2.3 Bump protocol to 9 and reject protocol 8 before mutation; verify the mismatch tests pass

## 3. Guide and OpenSpec

- [x] 3.1 Update the copyable supervisor loop to stop on exit 5 while rearming exit 3; verify guide assertions pass
- [x] 3.2 Sync lifecycle and CLI behavior into canonical specs and archive the completed change; verify `openspec validate --all` and an empty active change list

## 4. Integration validation

- [x] 4.1 Run formatting, Clippy with warnings denied, and the complete test suite; verify all commands pass
