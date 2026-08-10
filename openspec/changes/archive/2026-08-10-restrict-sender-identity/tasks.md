# Tasks — restrict `--sender` to configured agent identity

1. [x] TDD: rewrite `test_send_sender_mismatch_warns_on_stderr` →
   `test_send_sender_mismatch_rejected` (text mode): mismatched `--sender`
   exits non-zero, sends nothing, stderr names both identities.
2. [x] TDD: rewrite `test_send_sender_mismatch_json_signal` →
   `test_send_sender_mismatch_json_error`: `--json` mismatch emits
   `{"error": ..., "code": "SENDER_MISMATCH"}` on stderr, exit 1, no message
   sent; matching `--sender` still succeeds with no mismatch fields.
3. [x] `cmd_send`: replace the interim warning block with `fail(..., "SENDER_MISMATCH")`.
4. [x] `send_content`: drop `configured_sender` parameter and the
   `sender_mismatch`/`configured_sender` JSON fields.
5. [x] `cargo fmt --check` + `cargo test` green (132/132).
6. [x] PR vs main referencing issue #57 + openspec; merge after green.
7. [x] Close issue #57 with the decision record.
