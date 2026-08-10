# Dedicated exit code 3 for benign blocking timeouts

## Why

All blocking-wait timeout paths exit with clap's usage-error code **2**
(`wait <sess>` timeout, `wait --new-session` timeout, `send --wait` timeout),
even though a timeout is not a usage error — the command was called correctly,
it just found nothing in time. Scripts and agents that poll with
`tala wait --timeout N` cannot distinguish "I called it wrong" (2) from
"nothing arrived yet" (2): both look identical to a caller. `listen`/`stream`
timeouts already exit 0, making the family inconsistent.

Cycle-05 decision recorded in the backlog (B011/B018): benign timeouts get a
dedicated exit code, **3** (success paths stay 0; usage errors stay clap's 2;
real errors stay 1). This change implements that decision.

## What Changes

- New constant `EXIT_TIMEOUT: i32 = 3` in `src/cli.rs`.
- The six benign-timeout exit paths switch from `process::exit(2)` to
  `process::exit(EXIT_TIMEOUT)`:
  - `send --wait` timeout (text + JSON)
  - `wait <sess>` timeout (text + JSON)
  - `wait --new-session` timeout (both the dedicated command and the
    no-active-session fallback inside `cmd_wait`)
- Help text for `send` and `wait` documents the exit-code contract
  (0 = success, 3 = benign timeout, 2 = usage error, 1 = error).

## Capabilities

### New Capabilities
- `scriptable-timeouts`: callers can reliably detect benign timeouts via
  exit code 3.

### Modified Capabilities
- `messaging`: timeout semantics unchanged; only the exit code differs.
- `cli-docs`: exit-code contract now documented in `--help`.

## Impact

- Backlog: fixes B011/B018 (P2, decision recorded cycle-05).
- Files: `src/cli.rs` only (constant + 6 exits + 2 help strings);
  3 new e2e tests.
- Compatibility: any script that previously matched `!= 0` for timeouts is
  unaffected; scripts that matched `== 2` specifically would now see 3
  (the intended correction). No API/server changes.
