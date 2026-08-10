## 1. Rename `Chat` to `Send`, swap primary/alias

- [x] 1.1 Rename enum variant `Chat` → `Send`, set `#[command(name = "send")]`, remove `alias = "send"`
- [x] 1.2 Update help text on `Send` variant

## 2. Remove `Start` command, fold into `Send`

- [x] 2.1 Remove `Start` enum variant and its after_help
- [x] 2.2 Remove `Commands::Start` dispatch branch from `run()`
- [x] 2.3 Add `--name` / `-n` flag to `Send` variant
- [x] 2.4 Update `cmd_send` to accept `--name` parameter and pass it through to auto-create
- [x] 2.5 Add auto-create logic to `cmd_send` when no message and no active session (replaces bare `start`)
- [x] 2.6 Add error in `cmd_send` when no message supplied and active session exists
- [x] 2.7 Delete `cmd_start` function

## 3. Rename `Recap` to `History`

- [x] 3.1 Rename enum variant `Recap` → `History`, update help text
- [x] 3.2 Update `Commands::Recap` → `Commands::History` in dispatch

## 4. Rename `WhatsUp` to `Check`

- [x] 4.1 Rename enum variant `WhatsUp` → `Check`, update help text
- [x] 4.2 Update `Commands::WhatsUp` → `Commands::Check` in dispatch

## 5. Remove deprecated `Follow`, `Watch`, `Observe`

- [x] 5.1 Remove `Follow` enum variant
- [x] 5.2 Remove `Watch` enum variant
- [x] 5.3 Remove `Observe` enum variant
- [x] 5.4 Remove their dispatch branches from `run()`
- [x] 5.5 Remove `deprecation_warning()` function

## 6. Remove deprecated `--file` flag

- [x] 6.1 Remove the hidden `--file` arg from `Send` variant
- [x] 6.2 Clean up `--file` handling in `cmd_send` function body

## 7. Remove `--cursor` and `--new` flag aliases

- [x] 7.1 Remove `alias = "cursor"` from `--since` arg on `Wait` variant
- [x] 7.2 Remove `alias = "cursor"` from `--since` arg on `History` (`Recap`) variant
- [x] 7.3 Remove `alias = "new"` from `--new-session` arg on `Wait` variant

## 8. Rename `chit_dir` to `tala_dir`

- [x] 8.1 Rename variable `chit_dir` → `tala_dir` in `cmd_init`

## 9. Remove deprecated tests

- [x] 9.1 Remove `test_follow_alias_still_works` test
- [x] 9.2 Remove `test_file_deprecation_warning` test
- [x] 9.3 Update any tests referencing `start`

## 10. Update generated skill/docs

- [x] 10.1 Update embedded SKILL.md in `install_opencode_skills()` with new command names
- [x] 10.2 Update embedded command.md in `install_opencode_skills()`

## 11. Update README.md

- [x] 11.1 Update command table: `send` as primary, `recap`→`history`, `whatsup`→`check`, remove `start`
- [x] 11.2 Update examples to use `send` instead of `chat`/`send`

## 12. Update eval scenario

- [x] 12.1 Update `eval/scenarios/observe.md`: `tala observe` → `tala listen`

## 13. Verify

- [x] 13.1 Run `cargo build` to verify compilation
- [x] 13.2 Run `cargo test` to verify tests pass
