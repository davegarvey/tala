# Private agent selector files

## Why

Tala currently requires agents to keep a private handle in an invocation flag or shell environment. Agents whose shell state does not persist can lose that selection between commands. A protected selector file lets the handle stay outside project files and command output.

## What Changes

- Add `--agent-file=<path>` and `TALA_AGENT_FILE`, with explicit CLI selectors taking precedence over environment selectors and conflicting sources failing clearly.
- Add `register --save-handle-to=<path>` to create a new private selector file and omit the handle from successful output. If file creation fails after registration, return the handle once so the identity remains usable.
- Add private selector-file read and create-only write behavior on Unix and Windows, with updates to CLI help, the guide, and OpenSpec scenarios.
- Add end-to-end and platform-specific security coverage.

## Non-goals

## Non-Goals

- Store multiple identities in one selector file.
- Mutate or repair parent-directory permissions.
- Support selectors on platforms where Tala cannot verify private file access.
