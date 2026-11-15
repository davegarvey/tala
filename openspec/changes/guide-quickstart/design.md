# Design

## Context

See proposal.md for motivation. The CLI currently gives its global format field a JSON default, and `main` selects rendering before command dispatch. `integration::guide` returns the entire embedded Markdown file without a topic; named topics are extracted at level-two headings. Existing tests place supervisor assertions inside listening tests.

## Goals / Non-Goals

**Goals:** Select guide rendering without losing explicit global flag precedence; keep one authoritative reference and small, independently useful topics.

**Non-Goals:** Changing daemon messages, acknowledgement semantics, generated integration installation policy or harness configuration.

## Decisions

1. Represent an omitted format distinctly from an explicit selection in parsed CLI state (for example `Option<Format>`). Resolve omission to text for Guide and JSON otherwise. Keep pre-parse usage errors governed by the existing explicit-format scanner; this avoids guessing commands from unparsed positional arguments. Explicit flags work before or after the subcommand. Update global help to explain this exception.
2. Put a quickstart section at the beginning of the bundled Markdown. Bare guide returns only that section with topic `quickstart`; `all` returns the whole document. Keep topic extraction in one place rather than maintaining separate, divergent onboarding instructions. Add `harness` to the topic table and parser; move the loop and its supporting explanation into that section. Preserve ordinary wait semantics and cursor guidance in listening.
3. Quickstart uses copyable registration and identity selection examples, then peer discovery, send with `--request`, inbox and reply, plus wait/fetch/resume guidance. Include handle privacy, receipt versus completion and work-boundary checks. End with short topic descriptions and command-help discovery. Enforce the content budget with a focused regression.
4. Top-level help points to bare guide. Keep the post-registration `next: tala guide listening` hint: identity setup is already complete there and listening is the next useful subject. Update other topic lists and format descriptions found during implementation.

## Risks / Trade-offs

- Scripts parsing default guide JSON break → document `tala guide all --format=json` as the previous full-content equivalent and use a breaking conventional commit.
- Trimming listening may hide essential wait semantics → retain practical wait/fetch/resume instructions and move only supervisor implementation details.
- A content limit encourages cryptic prose → defer advanced material to topics and flags to command help, retaining complete essential explanations.

## Migration Plan

Implement and validate on a feature branch. Sync delta specs and archive in the implementation PR. No wire protocol bump is needed because these commands are local. Rollback restores prior CLI rendering/content selection; no stored state needs migration. After a merge, follow AGENTS.md to build, install and verify main, reporting waiter shutdown if the daemon is stopped.
