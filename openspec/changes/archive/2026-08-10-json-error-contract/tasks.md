# json-error-contract (B031 + B030) — tasks

## 1. Route cmd_send error paths through fail() (src/cli.rs)

- [x] 1.1 "Nothing to send" (`anyhow::bail!` at ~cli.rs:817) → `fail(json_output, msg, "NOTHING_TO_SEND")`; keep the hint but relabel `Active session:` → `Open session:`
- [x] 1.2 "Message cannot be empty." (`anyhow::bail!` at ~cli.rs:848) → `fail(json_output, msg, "EMPTY_MESSAGE")`

## 2. Tests (tests/e2e.rs, write FIRST)

- [x] 2.1 `send --json` with no content → exit 1, stderr parses as JSON with `code == "NOTHING_TO_SEND"`, hint text contains "Open session"
- [x] 2.2 `send --json ""` → exit 1, stderr JSON with `code == "EMPTY_MESSAGE"`
- [x] 2.3 human-mode `send` (no content, no session) → exit 1, stderr contains "Nothing to send" and "Open session", and NOT "Active session:"
- [x] 2.4 regression: `send --json` to closed session still emits `SESSION_CLOSED` JSON

## 3. Verify

- [x] 3.1 cargo fmt --check + clippy -D warnings clean
- [x] 3.2 cargo test green (offline after one fetch)
- [x] 3.3 live check vs shared daemon (both --json error shapes + human hint)
