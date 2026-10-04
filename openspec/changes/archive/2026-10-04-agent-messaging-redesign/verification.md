# Verification

Local macOS checks: cargo build; cargo test (7 unit and 23 end-to-end tests); cargo fmt --all -- --check; cargo clippy --all-targets -- -D warnings; git diff --check. All passed.

Real-agent usability: see eval/reports/2026-10-04-agent-interface.md. Two runtime subagents completed same-project and cross-project exchanges CLI-only, with no errors or internal storage reads. This does not validate live Claude hooks or Linux/Windows runners. CI now covers those operating systems.

Sync: five replacement capabilities added; every requirement in the eighteen superseded session-oriented capabilities explicitly removed through delta specs. Empty superseded capability directories removed. Installation contract preserved. Main specs validated before archive.

Upgrade: breaking protocol 2 and SQLite schema 1; old JSON and project identity/cursor files are left intact with no guessed import. Stop an old daemon with its matching binary first. This feature branch is for review; no production install or merge is part of verification.
