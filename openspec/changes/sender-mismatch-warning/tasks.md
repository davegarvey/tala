# Warn when `--sender` differs from configured identity — tasks

## 1. CLI: mismatch detection + warning (src/cli.rs)

- [ ] 1.1 In `cmd_send`, after content resolution: compute
      `override_name = sender_override` and
      `configured = store::get_sender_name(None)` (project config name, falling
      back to directory name)
- [ ] 1.2 When `sender_override.is_some() && override != configured`: emit
      `Warning: sending as '<override>' which differs from this project's
      configured agent '<configured>'. Recipients will see a spoofed sender
      identity.` to stderr (all modes: text, --json, --quiet)
- [ ] 1.3 Thread `sender_mismatch: Option<(String, String)>` into `send_content`;
      on the non-wait JSON success path, merge `"sender_mismatch": true` and
      `"configured_sender": "<configured>"` into the printed response object
- [ ] 1.4 No warning/fields when override matches configured identity or is absent

## 2. Help text

- [ ] 2.1 `--sender` arg help: append "(warns if it differs from the configured
      agent name)"

## 3. Tests (tests/e2e.rs, written first)

- [ ] 3.1 Test: project with `init <name>`, `send --sender <different>` →
      stderr contains "Warning: sending as" + the different name; exit 0
- [ ] 3.2 Test: `send --sender <configured-name>` → stderr does NOT contain
      "Warning: sending as"
- [ ] 3.3 Test: `send --sender <different> --json` → stdout JSON has
      `"sender_mismatch": true` and `"configured_sender": "<name>"`
- [ ] 3.4 Test: `send --sender <configured> --json` → stdout JSON has no
      `sender_mismatch` field (or false)
- [ ] 3.5 Regression: existing `--sender` tests still pass (warning on stderr is
      harmless)

## 4. Verify

- [ ] 4.1 `cargo fmt --check` clean
- [ ] 4.2 `cargo clippy -- -D warnings` clean
- [ ] 4.3 Full `cargo test` green
- [ ] 4.4 Live sandbox check: `--sender spoofed-agent` warns; `--sender alpha`
      (matching config) does not
