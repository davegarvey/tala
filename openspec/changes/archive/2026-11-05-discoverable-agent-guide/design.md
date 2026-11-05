# Design

## Context

The Markdown guide is already compiled into the CLI with `include_str!` and written to project instructions by `tala init`. CLI success results are JSON by default; `--human` renders text. `tala wait` is being added separately, so this change documents its intended notification flow but should be rebased after that command lands.

## Goals / Non-Goals

**Goals:**
- Reuse the bundled guide as the sole full-guide content source.
- Provide stable topic slugs and a read-only command that works without daemon state.
- Keep registration output concise and avoid repeating credentials.

**Non-Goals:**
- Installing user-level agent skills or writing files from `tala guide`.
- Implementing the unified `tala wait` command.

## Decisions

- Add explicit topic markers or a static extraction map for the five public slugs rather than deriving public names from current Markdown headings. This keeps the documented topic API stable while allowing the full guide to retain its natural section structure.
- Return `{topic, topics, content}` in the normal success envelope. For the full guide, `topic` is `all`; for a selected topic it is its slug. `content` is Markdown. `--human` prints only `content`.
- Set registration's `next` to `tala guide listening`; it exposes the listener setup without putting the private handle into a suggested command.
- Keep the unified wait instructions in the bundled guide canonical and use short pointers in help and README. The listening topic must describe `tala wait`, then the exact source-specific fetch command, then restarting `tala wait` after handling the event.

## Risks / Trade-offs

- Topic extraction can become stale if guide sections move. Add tests for all five slugs and expected key content.
- `tala wait` behavior may change before merge. Rebase after #113 and verify the instructions against the merged implementation before opening the PR.
- **Optional skill installation** → defer it; it writes to user-level, harness-specific locations and needs separate overwrite and path rules.
