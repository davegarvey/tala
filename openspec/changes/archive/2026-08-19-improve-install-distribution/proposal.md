## Why

Tala now has versioned GitHub release binaries and a published `tala-cli`
crate, but the documented install paths are not consistently usable. The
`cargo binstall` metadata does not match the release asset names, source
installation is not pinned or upgrade-safe, and release automation does not
verify that published artifacts can actually be installed.

## What Changes

- Align `cargo binstall` package URL and extraction metadata with the release
  archive names and target platforms.
- Add release verification for artifact naming, checksums, extraction, and
  `tala --version` execution.
- Update installation and upgrade documentation with reproducible commands,
  PATH expectations, crates.io installation, and direct release alternatives.
- Keep the existing GitHub Actions Trusted Publishing flow and document the
  supported installation channels.
- Make no changes to the CLI wire protocol, daemon behavior, or project setup
  semantics.

## Capabilities

### New Capabilities

- `installation`: Define supported Tala installation channels, release
  artifact compatibility, and verification expectations.

### Modified Capabilities

None.

## Impact

- Affected metadata: `Cargo.toml` `cargo-binstall` configuration.
- Affected automation: GitHub release packaging and verification workflow.
- Affected documentation: README install and upgrade instructions.
- No runtime dependencies, protocol changes, or user data migrations.
