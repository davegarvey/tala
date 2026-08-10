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

## Evaluation

E2E evaluation scenarios live in `eval/scenarios/` (see `eval/README.md` for
orchestration). They are manually orchestrated — no autonomous loop, no
commits or PRs from evals. Findings feed OpenSpec change proposals that a
human reviews.
