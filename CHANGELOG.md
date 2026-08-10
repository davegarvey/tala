# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.29.2] - 2026-08-10

### Fixed

- plateau validation findings — stale-active wait resilience, use help drift, pending json content (#82)
- plateau polish — listen exit codes, truthful banner, spec/doc sync (#81)

## [0.29.1] - 2026-08-10

### Fixed

- make listen trustworthy as the monitor command (B046) (#80)

## [0.29.0] - 2026-08-10

### Added

- narrow CLI surface to the golden path (cycle-19 evals) (#78)

### Changed

- archive cycle-18 openspec changes (eval feedback fixes, agent messaging docs)

## [0.28.1] - 2026-08-10

### Changed

- agent messaging golden path — handshake example, input-method guide, intent precedence (#65) (#76)
- Merge pull request #71 from davegarvey/docs/openspec-merge-ordering
- consolidate sync/archive into single workflow step
- state sync+archive must land in the implementation PR
- Merge pull request #70 from davegarvey/chore/openspec-archive-cleanup
- retro-sync archived delta specs into main specs
- archive completed openspec changes and sync delta specs

### Fixed

- cycle-18 eval feedback — session create --json output, reply correlation warning, wait-new session name (B039-B041) (#75)

## [0.28.0] - 2026-08-10

### Added

- adopt A2A principles — message parts, idempotent sends, daemon version checks

### Changed

- replace eval loop framework with lightweight scenario definitions (#69)
- Revert "chore: replace eval loop framework with lightweight scenario definitions"
- replace eval loop framework with lightweight scenario definitions

## [0.27.4] - 2026-08-10

### Fixed

- wait --new-session falls back to participated sessions with unread incoming (#68)

## [0.27.3] - 2026-08-10

### Changed

- intent-protocol: message intent metadata, reply correlation, and waiters registry (#40) (#66)

### Fixed

- print unread-session hint on send --wait timeout (v6 rerun finding) (#67)

## [0.27.2] - 2026-08-07

### Fixed

- restrict --sender to the configured agent identity (B004, #57) (#64)

## [0.27.1] - 2026-08-07

### Fixed

- die quietly on broken pipe instead of panicking (B038) (#63)
- reject duplicate session names on create and rename (B017) (#61)

## [0.27.0] - 2026-08-07

### Added

- warn when --sender differs from configured agent identity (B004 interim) (#58)
- show active daemon home in status; warn when TALA_HOME is unset (B036) (#56)

## [0.26.1] - 2026-08-07

### Fixed

- resolve session names for send/history/close/rename/wait (B035) (#55)

## [0.26.0] - 2026-08-07

### Added

- sender read receipts — daemon-side read_by exposed in list (B021) (#54)

## [0.25.5] - 2026-08-07

### Fixed

- exit code 3 for benign wait/send timeouts (B011, B018) (#48)

## [0.25.4] - 2026-08-07

### Fixed

- wait --new-session delivers freshest never-seen session, not stale backlog (B029) (#51)

## [0.25.3] - 2026-08-07

### Fixed

- replace global read cursor with per-session cursors (#46)

## [0.25.2] - 2026-08-07

### Fixed

- show connection status and message tally in listen/stream (B007) (#59)
- history/wait --limit returns newest N messages, not oldest (B016) (#53)
- --json error paths emit JSON; honest 'Open session' hint in send (B031, B030) (#52)
- close of active session clears active marker from all paths (B028) (#50)
- reject unknown send flags instead of silently misrouting (B026, B015) (#49)
- persist message history across daemon restarts (B024, B027) (#47)
- disambiguate list status column, empty-history note, reopen no longer steals active session (#45)

## [0.25.1] - 2026-08-06

### Changed

- Fix daemon crash recovery and shell injection UX issues (#39)
- cli-cmd-rename: simplify and clean up command surface
- fix-eval-cross-project: implement eval fix loop 0 (#36)
- fix state consistency bugs 0 (#35)
- fix state consistency bugs 0 (#34)
- fix state consistency bugs 0 (#33)
- fix loop 0 0 (#32)
- fix-send-discover-agents: implement fixes (#31)
- Merge pull request #30 from davegarvey/eval-fix-loop-0
- eval fix loop 0: implement fixes
- eval fix loop 0: implement fixes
- eval fix loop 0: implement fixes
- Merge pull request #29 from davegarvey/eval-fix-loop-0
- eval fix loop 0: implement fixes
- eval fix loop 0: implement fixes
- eval fix loop 0: implement fixes
- Rename observe→listen and follow→watch, add chit agents, improve chit wait UX
- session reopen, close --quiet, use-on-closed hint, stream alias
- Fix rename tests to be CI-safe (explicitly set name before testing overwrite rejection)
- chit send auto-create, --stdin flag, idle timeout, rename conflict detection
- Fix clippy warnings, add clippy to pre-commit hook
- Add cargo fmt pre-commit hook (#28)
- Add cargo fmt pre-commit hook (.githooks/pre-commit)
- Clean up eval docs, inline feedback, and code formatting (#27)
- Clean up eval docs, scenario feedback format, and code formatting
- Fix eval cd instructions, sync docs to current behavior (#26)
- Fix eval cd instructions, sync docs to current behavior (#26)
- Document full eval loop with OpenSpec + CI in AGENTS.md and chit-eval skill
- Fix observe eval feedback: session management, CLI ergonomics, observe timeout (#25)
- Add /api/sessions/wait-all endpoint and multi-session chit wait (#24)
- Fix duplicate message on chit start, remove auto-active session switch, improve eval reliability (#23)

### Fixed

- make wait --new-session deliver pre-existing incoming sessions (#44)
- align opencode docs with real CLI surface and make stop portable (#43)
- remove eval loop framework (#42)

## [0.25.0] - 2026-07-17

### Added

- tala-eval skill for coding agent eval workflow

## [0.4.0] - 2026-07-14

### Added

- auto-detect opencode harness in init instead of --opencode flag

## [0.3.0] - 2026-07-14

### Added

- rename send command to chat, keep send as alias

### Changed

- fix wording in install instructions
- update install instructions for tala-cli rename, add binstall metadata

## [0.2.2] - 2026-07-14

### Fixed

- rename crate to tala-cli to avoid crates.io conflict

## [0.2.1] - 2026-07-14

### Fixed

- handle stop on non-unix platforms properly to fix Windows build

## [0.2.0] - 2026-07-14

### Added

- add README with quick start, usage, and install instructions

### Changed

- Simplify release to direct-push flow
- Fix formatting for CI
- Add CI and release workflows
- Initial commit: tala v0.1.0

### Fixed

- trigger release on push to main (not just PR merge)
