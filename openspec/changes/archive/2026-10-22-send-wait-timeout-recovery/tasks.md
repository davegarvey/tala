# Tasks

## 1. Timeout recovery behavior

- [x] 1.1 Add the correlated inbox recovery command to `send --wait` timeout results and verify its JSON envelope includes the stored request ID and original timeout.
- [x] 1.2 Render the recovery command in human mode and verify timeout guidance says not to resend; generic inbox/board timeout rendering remains accurate.
- [x] 1.3 Add or update focused e2e coverage proving a timed-out request can be resumed and receives its late reply without creating a second request.

## 2. Validation and documentation

- [x] 2.1 Update the CLI spec and verify `openspec validate --all` passes.
- [x] 2.2 Run `cargo fmt --check`, `cargo test`, and clippy with warnings denied.
