## 1. Integration Compatibility Model

- [x] 1.1 Define the pair-level integration status model for absent, current, stale, incompatible, and unknown states, including partial files and inconsistent metadata.
- [x] 1.2 Resolve one project root from the nearest ancestor containing `.tala/config.json` or `.opencode/`, with current-directory fallback and no sibling/global scanning.
- [x] 1.3 Parse and validate both documents' generated/minimum CLI metadata using the installed binary version as the authority.
- [x] 1.4 Add unit coverage for state precedence, below-minimum versions, mixed document versions, and missing/invalid metadata.

## 2. Runtime Diagnostics

- [x] 2.1 Add a non-blocking compatibility preflight to normal command dispatch, excluding help, version, init, and explicit integration-check/refresh flows.
- [x] 2.2 Emit at most one concise compatibility warning to stderr while preserving normal stdout and `--json` output.
- [x] 2.3 Add a fallible top-level parse path that preserves Clap's usage text and exit status while adding stale-integration guidance to unknown-subcommand errors.

## 3. Safe Integration Management

- [x] 3.1 Add `tala init --check` and `tala init --refresh` with mutually exclusive argument validation, including rejection of positional identity names.
- [x] 3.2 Add `--json` output for check and refresh with stable status, project-root, version, and file-path fields.
- [x] 3.3 Make ordinary `tala init` create missing integration files while preserving existing skill and command documents.
- [x] 3.4 Make explicit refresh render and validate both current templates before replacing the pair, without changing `.tala/config.json`.
- [x] 3.5 Make pair replacement recover from a failure after the first destination changes, leaving both existing documents unchanged.
- [x] 3.6 Report integration state and refreshed file paths clearly in human-readable output and use the shared project root.

## 4. Documentation Surface Alignment

- [x] 4.1 Align the CLI implementation and help output with the explicitly reduced public surface, including removal of `stream` and `agents`.
- [x] 4.2 Retain machine-readable CLI compatibility metadata and concise agent guidance for checking `tala --help` and refreshing stale integration files.
- [x] 4.3 Align the canonical README, skill, and command templates with the intentionally reduced public CLI surface.
- [x] 4.4 Update documentation consistency checks so removed commands cannot remain in canonical integration documents.

## 5. Verification

- [x] 5.1 Add end-to-end coverage for current, stale, incompatible, unversioned, partial, mixed-version, and absent integrations.
- [x] 5.2 Add end-to-end coverage proving stale warnings use stderr and do not corrupt human or JSON stdout.
- [x] 5.3 Add end-to-end coverage for nested-directory root selection and no-op identity configuration behavior.
- [x] 5.4 Add end-to-end coverage for init preservation, check/refresh JSON output, argument conflicts, atomic refresh failure, and customized-file replacement.
- [x] 5.5 Add end-to-end coverage for unknown-command parsing before normal dispatch, including exit status and no-integration behavior.
- [x] 5.6 Run `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
