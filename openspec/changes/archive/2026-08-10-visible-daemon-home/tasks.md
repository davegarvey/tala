# Visible daemon home (B036) — tasks

## 1. CLI: status shows home (src/cli.rs)

- [x] 1.1 `cmd_status` text (running): print `  Home: <path>` after `Since:`
- [x] 1.2 `cmd_status --json` (running): add `home` + `tala_home_set` fields
- [x] 1.3 `cmd_status --json` (not running): add `tala_home_set` field (home exists)
- [x] 1.4 `cmd_status` text: when TALA_HOME unset and daemon running, print stderr
      warning `warning: TALA_HOME is not set — using default daemon home <path>`

## 2. Tests (tests/e2e.rs)

- [x] 2.1 status (running) text contains `Home:`
- [x] 2.2 status (running) `--json` has `home` and `tala_home_set: true` with
      TALA_HOME set, `tala_home_set: false` without
- [x] 2.3 status (not running) `--json` includes `home`
- [x] 2.4 default-home warning appears on stderr when TALA_HOME unset

## 3. Docs

- [x] 3.1 README command table: note `status` shows the active home
