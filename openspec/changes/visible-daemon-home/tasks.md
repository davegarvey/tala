# Visible daemon home (B036) — tasks

## 1. CLI: status shows home (src/cli.rs)

- [ ] 1.1 `cmd_status` text (running): print `  Home: <path>` after `Since:`
- [ ] 1.2 `cmd_status --json` (running): add `home` + `tala_home_set` fields
- [ ] 1.3 `cmd_status --json` (not running): add `tala_home_set` field (home exists)
- [ ] 1.4 `cmd_status` text: when TALA_HOME unset and daemon running, print stderr
      warning `warning: TALA_HOME is not set — using default daemon home <path>`

## 2. Tests (tests/e2e.rs)

- [ ] 2.1 status (running) text contains `Home:`
- [ ] 2.2 status (running) `--json` has `home` and `tala_home_set: true` with
      TALA_HOME set, `tala_home_set: false` without
- [ ] 2.3 status (not running) `--json` includes `home`
- [ ] 2.4 default-home warning appears on stderr when TALA_HOME unset

## 3. Docs

- [ ] 3.1 README command table: note `status` shows the active home
