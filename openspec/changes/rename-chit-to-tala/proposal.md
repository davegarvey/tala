## Why

The app has outgrown its original name "chit" (agent-to-agent chat). The new name "tala" reflects a broader vision — it means "to speak" or "to tell" in several languages, aligning with the tool's role as a communication layer for AI agents. The rename also avoids confusion with other projects named "chit" and gives the project a unique identity.

## What Changes

- Rename crate from `chit-cli` to `tala-cli`, binary from `chit` to `tala`
- Rename the root project directory from `chit` to `tala`
- Rename environment variable `CHIT_HOME` → `TALA_HOME`
- Rename runtime data directory `.chit/` → `.tala/`
- Rename all OpenCode agent/skill/command files and directories from `chit*` → `tala*`
- Update all source code references, help text, and error messages
- Update CI/CD artifact names and packaging scripts
- Update documentation (README, CHANGELOG, AGENTS.md)
- Update git remote origin URL to point to `davegarvey/tala`
- Skip historical openspec change files (archived, not worth updating)

## Capabilities

### New Capabilities
*(None — this is a rename, not a new feature)*

### Modified Capabilities
- `cli-ux`: All CLI help text, error messages, and app name strings updated from `chit` to `tala`
- `agent-discovery`: The `.opencode/skills/chit/` skill is renamed to `.opencode/skills/tala/`; the skill `name` field changes from `chit` to `tala`

## Impact

- **Breaking change**: Binary name changes from `chit` to `tala` — all scripts, aliases, and CI references must update
- **Breaking change**: Environment variable `CHIT_HOME` → `TALA_HOME`
- **Breaking change**: Runtime directory `.chit/` → `.tala/` — existing `.chit/` data is not migrated
- **Code**: All Rust source files across `src/`, `tests/e2e.rs`, `Cargo.toml`
- **Config**: `.gitignore`, `.chit/config.json`, `.versionrc.json`
- **Infrastructure**: CI workflows, release artifact names, packaging scripts
- **Tooling**: OpenCode skill files, agent config, eval framework
- **Docs**: README, CHANGELOG, AGENTS.md
