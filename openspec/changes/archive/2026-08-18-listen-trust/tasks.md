# Tasks — listen trust (B046)

## 1. timeout 0 honored

- [x] 1.1 cli.rs: `timeout.or(Some(60))` — Some(0) passes through as
      `timeout_secs=0` (daemon already treats 0 as no deadline)
- [x] 1.2 e2e (red): listen --timeout 0 stays connected and delivers a
      message sent after connect

## 2. Cursor advancement

- [x] 2.1 cli.rs: per delivered message event, write the session cursor
      (only when no explicit --since/--since_map replay mode)
- [x] 2.2 e2e (red): after listen delivers, `tala check` reports nothing new;
      a second listen doesn't replay

## 3. Lagged no longer silent

- [x] 3.1 api.rs: on Lagged(n) send an `overload` event with skipped count
- [x] 3.2 cli.rs: render `overload` to stderr ("missed N message(s) — run
      `tala check`")
- [x] 3.3 e2e: overload event rendered (simulate via unit-testable path or
      documented; at minimum the parser handles the event type)

## 4. Validation

- [x] 4.1 fmt + clippy clean; full e2e green
- [x] 4.2 Live: timeout-0 listen parks past 60s; delivered-then-check agrees
