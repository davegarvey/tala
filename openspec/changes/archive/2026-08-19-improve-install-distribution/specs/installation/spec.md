## Purpose

The installation capability defines reliable, verifiable ways to obtain the
Tala CLI from crates.io or versioned GitHub release artifacts.

## ADDED Requirements

### Requirement: Published installation channels

The project SHALL publish the `tala-cli` crate to crates.io and SHALL publish
versioned release archives for every supported release target. The archive
names SHALL identify the operating system and architecture and SHALL contain
the `tala` executable at a documented path.

#### Scenario: Install the latest crate release

- **WHEN** a user runs `cargo install tala-cli --locked --force`
- **THEN** Cargo SHALL be able to resolve and install the latest published
  `tala` executable from crates.io

#### Scenario: Download a supported release archive

- **WHEN** a user selects a supported target from a versioned GitHub release
- **THEN** the release SHALL provide an archive for that target
- **AND** extracting the archive SHALL produce an executable named `tala`
  or `tala.exe` at its expected archive path

### Requirement: Cargo binstall release resolution

The package metadata SHALL resolve each supported `cargo binstall tala-cli`
target to the matching versioned GitHub release archive without requiring a
source build. The resolved archive format and executable path SHALL match the
published asset for that target.

#### Scenario: Binstall resolves macOS ARM

- **WHEN** a user runs `cargo binstall --force tala-cli` on `aarch64-apple-darwin`
- **THEN** binstall SHALL request the versioned `tala-macos-aarch64` archive
- **AND** SHALL install the archive's `tala` executable

#### Scenario: Binstall resolves Linux targets

- **WHEN** a user runs `cargo binstall --force tala-cli` on a supported Linux
  architecture
- **THEN** binstall SHALL request the corresponding `tala-linux` archive
- **AND** SHALL install the archive's `tala` executable

#### Scenario: Binstall resolves Windows x86_64

- **WHEN** a user runs `cargo binstall --force tala-cli` on
  `x86_64-pc-windows-msvc`
- **THEN** binstall SHALL request the `tala-windows-x86_64.exe.zip` archive
- **AND** SHALL install `tala.exe`

### Requirement: Release artifact verification

The release workflow SHALL verify each produced archive before upload by
checking its expected executable, extracting it, and confirming that
`tala --version` reports the release version. Unix archives SHALL include a
SHA-256 checksum that matches the uploaded archive.

#### Scenario: Release archive has the expected executable

- **WHEN** the release workflow packages a target archive
- **THEN** the workflow SHALL fail if the expected `tala` or `tala.exe` entry is
  missing

#### Scenario: Release archive runs the tagged version

- **WHEN** the workflow extracts a packaged archive
- **THEN** executing its Tala binary with `--version` SHALL report the version
  being released

#### Scenario: Unix archive checksum is valid

- **WHEN** the workflow uploads a Unix archive and checksum file
- **THEN** the checksum verification SHALL pass before the upload completes

### Requirement: Installation documentation

The README SHALL document the supported crates.io, `cargo binstall`, source,
and GitHub release installation paths. It SHALL explain that Cargo-installed
binaries require the Cargo bin directory on `PATH`, provide an upgrade command,
and include a command for verifying the installed version.

#### Scenario: User follows the latest-install instructions

- **WHEN** a user follows the README's latest-install instructions
- **THEN** the instructions SHALL install or upgrade `tala`
- **AND** SHALL show how to confirm the resolved executable and version

#### Scenario: User follows a pinned source-install instruction

- **WHEN** a user needs a reproducible source installation
- **THEN** the README SHALL show a version-tagged, locked Cargo installation
  command
