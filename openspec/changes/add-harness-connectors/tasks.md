# Tasks

## 1. Verify harness mechanisms

- [ ] 1.1 In Claude Code (CLI and desktop Code tab), install a throwaway user-scope `PostToolUse` and `SessionStart` hook that emits a marker through `hookSpecificOutput.additionalContext`; verify the model repeats the marker, record the payload fields and the `compact` matcher's behaviour in `eval/results/connectors-spike/` (git-ignored)
- [ ] 1.2 Repeat 1.1 for Codex CLI and the Codex desktop app via `~/.codex/hooks.json`, including the review step and the `source: compact` session start; record payloads and whether `async: true` delays delivery
- [ ] 1.3 In OpenCode, write a throwaway plugin that appends a marker to `output.output` in `tool.execute.after`, and test session-created and compaction events for adding context; record which mechanisms reach the model and update design.md's OpenCode decision with the result

## 2. Connector catalogue and manifests

- [ ] 2.1 Define the manifest schema and loader in `src/connector.rs`, embedding `connectors/*/connector.json` with `include_str!`; verify with a unit test that every bundled manifest parses and declares its scopes, events, session key and render rule
- [ ] 2.2 Add `connectors/claude-code/`, `connectors/codex/` and `connectors/opencode/` manifests and the OpenCode plugin template from the task 1 findings; verify the templates render with the binary path and version substituted
- [ ] 2.3 Add the `connector` command group with `list` to `src/cli.rs`; verify `tala connector list` in JSON and text formats, the unknown-connector usage error (exit 2) and that `--help` shows connector commands only under `tala connector`

## 3. Install, remove and check

- [ ] 3.1 Implement the install planner: JSON merge for hook configuration, owned-file write for plugins, `--scope=user|project`, `--dry-run`, staged writes with rollback; verify with tests on fixture configurations containing unrelated hooks, an unparseable file, and a repeated install that reports `unchanged`
- [ ] 3.2 Report harness approval steps in install output (Codex review location); verify in the `codex` install test
- [ ] 3.3 Implement `remove` by ownership marker and `check` (current, stale version, missing binary, absent); verify install-then-remove restores semantically equal configuration and that a missing binary path reports stale with the install command as `next`
- [ ] 3.4 Confirm `tala init` still writes no harness configuration when `.claude/`, `.codex/` or `.opencode/` exist; verify with an e2e test

## 4. Hook runner

- [ ] 4.1 Implement payload parsing, session-key derivation and encoding, and the 0700 session directory under `$TALA_HOME/sessions/`; verify unit tests for missing keys, unsafe characters and long identifiers
- [ ] 4.2 Add a client mode that never spawns a daemon and enforces a two-second budget; verify the hook exits 0 with no output and starts no daemon when none is running
- [ ] 4.3 Implement session-start handling: registration instruction in initialised projects, `--agent-file` reminder for registered sessions and after compaction, silence elsewhere; verify that injected text never contains `th_` handles
- [ ] 4.4 Implement change-only attention with `notified.json` and the 15-minute repeat; verify e2e that a new request is reported once, not repeated within 15 minutes (using an injectable clock), repeated afterwards, and left unread
- [ ] 4.5 Render through each manifest's output rule and log errors to the size-capped `hook.log`; verify golden-output tests replaying the recorded payloads from task 1 for all three connectors
- [ ] 4.6 Verify the Codex desktop app runs project-scope hooks for a trusted project and record the answer in the README scope advice

## 5. Documentation and evaluation

- [ ] 5.1 Update README (Connectors section, scope advice, approval steps, removal) and the guide's `harness` topic; verify `tala guide harness` shows the connector instructions and the quickstart stays under 2,000 characters
- [ ] 5.2 Add `eval/scenarios/long-session.md`: two agents on 20–30 minute tasks, one request sent mid-task, run with and without a connector, recording time to notice, missed requests and lost identities; list it in `eval/README.md`
- [ ] 5.3 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`; verify all pass

## Workflow follow-up

- Install each connector on this machine for Claude Code desktop, Codex desktop and OpenCode CLI, then run the long-session scenario and report the results for review.
- Sync the delta specs into `openspec/specs/` and archive the change on the feature branch before merging.
