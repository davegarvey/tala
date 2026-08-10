# tala

Agent-to-agent messaging for AI coding tools. Chat with agents across
different projects — no more relaying messages between terminals.

## Development

```bash
cargo build
cargo test
```

Pre-commit hooks require `cargo fmt --check` and clippy with `-D warnings`.

## CLI design

When creating or modifying CLI tools, follow the [Command Line Interface Guidelines](https://clig.dev/):
`verb-noun` subcommands, consistent flags (`--help`, `--version`), `=` for flag values,
meaningful exit codes, stderr for logs/stdout for data, and `--` to separate options
from positional arguments.

## Commits

Use conventional commits (`feat:`, `fix:`, etc.) — the release pipeline bumps
the version from them, and non-conventional messages silently skip releases.

## Wire protocol

The CLI↔daemon wire protocol has a `PROTOCOL_VERSION` (`src/models.rs`). Bump
it on any incompatible wire change (new required request fields, changed
message shapes); the CLI refuses commands against a mismatched daemon and
read-only commands warn. `--json` output keeps the legacy `content` field on
messages alongside `parts` for older clients.

## OpenSpec workflow

All feature work follows the OpenSpec change workflow in `openspec/` (see
`.opencode/skills/openspec-*` and `.opencode/commands/opsx-*`):

1. **Create** a change (`openspec-new-change`): proposal → delta specs → design → tasks.
2. **Implement** the change, checking off tasks as they land.
3. **Sync** the change's delta specs into main specs (`openspec-sync-specs`) — required
   before archiving so `openspec/specs/` stays the accumulated source of truth.
4. **Archive** completed changes (`openspec-archive-change` / `openspec-bulk-archive-change`),
   moving them to `openspec/changes/archive/YYYY-MM-DD-<name>/`. Never archive without syncing
   delta specs first, unless the change is superseded (document why).
5. Sync and archive on the feature branch, in the same PR as the implementation, so `main` is
   clean at every merge. If a reviewer objects to mixing docs with code, an immediate doc-only
   follow-up PR is acceptable — but open it when the code PR merges, not later.
6. Completed-but-unarchived changes are technical debt — archive promptly; keep `openspec list`
   empty and `openspec validate --all` green.

## Evaluation

E2E evaluation scenarios live in `eval/scenarios/` (see `eval/README.md` for
orchestration). They are manually orchestrated — no autonomous loop, no
commits or PRs from evals. Findings feed OpenSpec change proposals that a
human reviews.
