## Why

The tala CLI has accumulated inconsistent command names, deprecated aliases, and overlapping functionality (`start` vs `send --wait`) that make the interface harder to learn and use. Cleaning this up before v1.0 reduces surface area and aligns commands with clear, conventional verbs.

## What Changes

All changes are **BREAKING** — no deprecated backward-compatibility path:

- **`chat` → `send`**: Rename primary command from `chat` to `send`. Drop the `chat` alias entirely.
- **Remove `start`**: Fold into `send`. `send` with no message and no session auto-creates a session (replaces bare `tala start`). Add `--name` flag to `send` for session naming.
- **`recap` → `history`**: Rename command.
- **`whatsup` → `check`**: Rename command for non-blocking "anything new?" check.
- **Remove deprecated commands**: Delete hidden `follow`, `watch`, `observe` variants and their dispatch code.
- **Remove deprecated `--file` flag**: Delete the hidden `--file` flag on `send`; keep `--message-file`.
- **Remove deprecated flag aliases**: Drop `--cursor` (alias for `--since`) and `--new` (alias for `--new-session`).
- **`chit_dir` → `tala_dir`**: Rename variable in `cmd_init`.

## Capabilities

### New Capabilities
- `cli-commands`: Definitive command set for tala CLI — names, flags, and behavior.

### Modified Capabilities
- (none — this is the first formal spec)

## Impact

- `src/cli.rs`: Major rework of `Commands` enum, dispatch in `run()`, and several handler functions. Remove `cmd_start`, `deprecation_warning()`. Rename `Chat` → `Send` variant.
- `tests/e2e.rs`: Remove two tests (`test_follow_alias_still_works`, `test_file_deprecation_warning`). Update tests that reference `start`.
- `README.md`: Update command table and examples.
- Generated skill/command docs in `install_opencode_skills()`: Update embedded markdown.
- `eval/scenarios/observe.md`: Update `tala observe` → `tala listen` reference.
