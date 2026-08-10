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
