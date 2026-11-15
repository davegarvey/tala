# Tasks

## 1. Guide content and selection

- [ ] 1.1 Add the under-2,000-character quickstart, separate harness topic and explicit all selection; update topic parsing and extraction tests to verify essential content, topic isolation and full-reference completeness.
- [ ] 1.2 Move existing harness regression assertions to the harness topic and retain listening semantics checks; verify focused guide tests pass without creating an identity, daemon or state.

## 2. Rendering and discovery

- [ ] 2.1 Resolve omitted format to text for guide and JSON otherwise, preserving explicit selection before and after subcommands; add parsing and E2E coverage for quickstart/topics/all in default, JSON and text modes, plus unchanged structured usage errors.
- [ ] 2.2 Update top-level and guide help, topic lists and relevant output-format documentation; verify help points to the quickstart and documents the format exception and migration command, while registration keeps its listening hint.

## 3. Integration verification

- [ ] 3.1 Run cargo fmt --check, cargo clippy --all-targets -- -D warnings, cargo test and openspec validate --all; resolve failures and manually inspect isolated-home guide output for readability and the content budget.

## Workflow follow-up

- Sync delta specs into main specs and verify the merge, then archive this change on the feature branch before opening the implementation PR.
- Use a breaking conventional commit and explain `tala guide all --format=json` for scripts migrating from the old full JSON default.
- After merge, install and verify the merged main release as required by AGENTS.md; report any wait calls ended by daemon shutdown.
