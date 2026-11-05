# Tasks

## 1. CLI guide

- [x] 1.1 Add the local guide command and topic extraction; verify JSON, human, no-daemon and invalid-topic behavior with CLI tests.
- [x] 1.2 Add the registration `next` field and human output pointer; verify the handle is not repeated in `next`.

## 2. Guidance and specification

- [x] 2.1 Update bundled guide, help and README with the canonical `tala wait` notification, source fetch and restart workflow; verify command examples against #113 after rebase.
- [x] 2.2 Sync the `agent-cli` delta into the main spec, archive this change, and verify `openspec validate --all`.

## 3. Integration checks

- [x] 3.1 Run formatting, tests, and Clippy with warnings denied; verify the CLI help and all guide topics.
