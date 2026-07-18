# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
