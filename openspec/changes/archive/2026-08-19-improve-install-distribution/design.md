## Context

The repository publishes archives named `tala-linux-x86_64.tar.gz`,
`tala-linux-aarch64.tar.gz`, `tala-macos-aarch64.tar.gz`, and
`tala-windows-x86_64.exe.zip`. The current binstall template uses the crate
name and full Rust target, so it does not resolve those assets. The release
workflow already has the target matrix and crates.io Trusted Publishing; this
change should improve the distribution contract without changing runtime code.

## Goals / Non-Goals

**Goals:**

- Make `cargo binstall tala-cli` resolve the existing supported release assets.
- Fail releases when an archive is malformed or contains the wrong version.
- Make install and upgrade instructions accurate and reproducible.
- Keep crates.io Trusted Publishing as the only automated crate-publish path.

**Non-Goals:**

- No daemon, wire protocol, or project initialization behavior changes.
- No rename of existing release assets.
- No new shell installer or unsigned third-party install script.
- No promise of prebuilt artifacts for targets absent from the release matrix.

## Decisions

### Map binstall metadata to release naming

Use the literal binary name together with binstall's operating-system,
architecture, binary-extension, and archive-suffix variables. This maps the
existing release names without coupling them to the crate name or full Rust
target triple. Keep the Windows archive-format override and use the binary
extension in both the URL and installed path.

Alternative considered: rename all release assets to the default crate/target
scheme. Rejected because existing release links are already public and the
short names are easier to read.

### Verify artifacts in each build matrix job

Each platform job will run the built binary, package it, inspect the archive,
extract it into a temporary directory, and run the extracted binary. Unix jobs
will verify the generated SHA-256 file before uploading. This catches packaging
and target/version mistakes close to their source and avoids relying only on a
later aggregate job.

### Keep a Linux binstall smoke test

After the crate publish and release assets are available, a release job will
install `cargo-binstall` and install the just-published Tala crate for the
Linux musl target. It will run `tala --version` and fail if binstall falls back
to an unexpected path or cannot resolve the release asset. Matrix archive
checks still validate the other supported platforms.

### Document channels by trust and reproducibility

README instructions will prefer the published crate for latest installs,
describe binstall as the prebuilt path, and show a version-tagged `--locked`
source command for reproducibility. Every path will end with `command -v tala`
and `tala --version` verification. Cargo PATH setup will be explicit instead
of implied.

## Risks / Trade-offs

- [Risk] A new binstall template can still drift from future asset naming →
  [Mitigation] keep the release matrix and template together, add the
  binstall smoke test, and fail archive verification before upload.
- [Risk] crates.io and GitHub release propagation can be briefly asynchronous
  → [Mitigation] the publish job waits for crates.io availability and the
  binstall smoke test runs only after publish and build jobs complete.
- [Risk] Unsupported targets have no prebuilt archive → [Mitigation] document
  the supported matrix and retain Cargo source installation as the fallback.

## Migration Plan

1. Merge the metadata, workflow verification, and README changes.
2. On the next version bump, verify all release jobs and the binstall smoke
   test before treating the new release channel as healthy.
3. Existing users need no data migration; they can upgrade with the documented
   force-enabled install command.
4. Rollback is a source change only: restore the previous metadata and
   verification steps without changing published versions.
