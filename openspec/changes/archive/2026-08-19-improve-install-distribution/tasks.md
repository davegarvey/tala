## 1. Align Installation Metadata And Documentation

- [x] 1.1 Update `Cargo.toml` `cargo-binstall` metadata to resolve the existing OS, architecture, archive, and executable names for every supported target.
- [x] 1.2 Update README quick-start and install sections with crates.io, binstall, pinned locked source, and GitHub release options.
- [x] 1.3 Document Cargo PATH requirements, upgrade commands, version verification, and the supported prebuilt target matrix.

## 2. Verify Release Artifacts

- [x] 2.1 Add Unix release-job checks for the built binary version, archive contents, extraction, executable version, and SHA-256 checksum.
- [x] 2.2 Add Windows release-job checks for the built executable version, ZIP contents, extraction, and executable version.
- [x] 2.3 Add a post-publish Linux `cargo binstall` smoke test for the just-published version and verify the installed `tala --version` output.
- [x] 2.4 Make release builds and publishing use the committed lockfile where supported.

## 3. Validate And Prepare Release

- [x] 3.1 Run package and metadata checks proving the crate is publishable and the binstall configuration is syntactically valid.
- [x] 3.2 Run formatting, tests, clippy, and strict OpenSpec validation.
- [x] 3.3 Sync the installation spec, archive the completed change, and prepare the pull request.
