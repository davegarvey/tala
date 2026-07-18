## 1. Rename observe to listen and follow to watch

- [ ] 1.1 Add `Listen` variant to Commands enum (primary), keep `Observe` as hidden alias with deprecation warning
- [ ] 1.2 Add `Watch` variant to Commands enum (primary), keep `Follow` as hidden alias with deprecation warning, keep `Stream` alias
- [ ] 1.3 Update dispatch in `run()`: `Listen` calls `cmd_listen`, `Observe` calls `cmd_listen` with deprecation warning
- [ ] 1.4 Update dispatch: `Watch` calls `cmd_watch`, `Follow` calls `cmd_watch` with deprecation warning, `Stream` calls `cmd_watch` with deprecation warning
- [ ] 1.5 Rename `cmd_observe` function to `cmd_listen`, add thin `cmd_observe` wrapper that prints deprecation warning
- [ ] 1.6 Rename `cmd_follow` function to `cmd_watch`, add thin `cmd_follow` wrapper that prints deprecation warning
- [ ] 1.7 Update help text descriptions for listen and watch
- [ ] 1.8 Update `chit --help` (about text mentioning observe)
- [ ] 1.9 Update README.md command table
- [ ] 1.10 Update SKILL.md template in cli.rs (the init skill text)
- [ ] 1.11 Update test references: rename test functions and command invocations in e2e.rs
- [ ] 1.12 Add tests for alias deprecation warnings

## 2. Improve chit wait without session (2+ session case)

- [ ] 2.1 In `cmd_wait` session resolution: when 2+ sessions found and no active session, list sessions with IDs/names/message counts and return instead of calling wait-all
- [ ] 2.2 Handle --json output for the 2+ session case: return `{"sessions": [...], "error": "..."}`
- [ ] 2.3 Update tests for new wait behavior

## 3. Add chit agents command

- [ ] 3.1 Add `AgentSummary` struct to models.rs with `sender`, `last_seen`, `message_count`
- [ ] 3.2 Add `GET /api/agents` handler to api.rs that iterates open sessions and aggregates sender stats
- [ ] 3.3 Add route registration for `/api/agents`
- [ ] 3.4 Add `Agents` variant to Commands enum with `--json` flag
- [ ] 3.5 Add `cmd_agents` implementation that calls `/api/agents` and formats output
- [ ] 3.6 Add dispatch in `run()`
- [ ] 3.7 Write tests for chit agents

## 4. Documentation and verification

- [ ] 4.1 Update `chit init` generated SKILL.md to reference listen/watch
- [ ] 4.2 Verify all tests pass with `cargo test`
