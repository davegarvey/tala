# Agents in sibling worktrees

Agents often work in parallel in separate linked worktrees of one repository. Checks that they share one project board while each message still records where it came from.

## Setup

```bash
git init repo && git -C repo commit --allow-empty -m init
git -C repo worktree add ../wt-feature -b feature
```

Start agent A in `repo` and agent B in `wt-feature`, sharing one isolated Tala home. Give each `docs/agent-guide.md`.

## Tasks

- A: Post on the project board which files you are changing.
- B: Before starting, read the project board and avoid A's files; post your own plan.

## Expected

- `whoami` reports the same `context.project` for both agents and different `context.worktree` values.
- Each agent sees the other's post on its default board, with the poster's worktree shown.
- A board in an unrelated repository stays separate.

## Baseline (0.34.0, 2026-10-12)

The worktree agent's project was the worktree path itself, so its default board showed none of the main checkout's posts.
