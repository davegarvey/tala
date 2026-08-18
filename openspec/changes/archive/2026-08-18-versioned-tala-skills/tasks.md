## 1. Define Versioned Templates

- [x] 1.1 Record the command/flag compatibility matrix for releases `0.25.0`, `0.25.1`, `0.27.3`, and `0.28.0`, and set the initial `tala_cli_min_version` to the evidenced current-skill floor of `0.27.3`.
- [x] 1.2 Add distinct `tala_cli_min_version` and generated-version placeholders to the canonical skill and command document frontmatter while preserving the existing skill-content version.
- [x] 1.3 Add agent guidance to the skill for checking `tala --version`, comparing the minimum semver, and distinguishing incompatibility from a stale-document warning.
- [x] 1.4 Document Semantic Versioning precedence, including prerelease and build metadata behavior, in the agent guidance.
- [x] 1.5 Replace the informal `v0.25+` compatibility text with wording consistent with the machine-readable compatibility floor.

## 2. Render Metadata During Initialization

- [x] 2.1 Implement a pure `render_integration_document`-style seam that replaces both version placeholders with the configured floor and running binary's package version.
- [x] 2.2 Make `tala init` render and validate both documents before writing either one, reject a minimum version above the generated version, fail clearly when a required placeholder is malformed, and use temporary files with atomic replacement per destination.
- [x] 2.3 Preserve existing project initialization behavior, including config preservation and skipping integration installation when `.opencode/` is absent.

## 3. Add Compatibility Coverage

- [x] 3.1 Update initialization tests to verify both installed documents contain valid compatibility metadata and the current package version.
- [x] 3.2 Update document consistency tests to validate required metadata keys, distinct skill-content versioning, and `tala --version` guidance.
- [x] 3.3 Add renderer unit coverage for missing or duplicated placeholders, invalid rendered versions, and the resulting initialization error.
- [x] 3.4 Verify legacy skill documents without the new metadata are identified as unversioned and needing refresh by the documented guidance.
- [x] 3.5 Add comparison fixtures for below-minimum, older-than-generated, equal, newer-than-generated, prerelease, and build-metadata versions.
- [x] 3.6 Verify a render failure leaves existing skill and command documents unchanged.
- [x] 3.7 Add CLI-output fixtures for a failed `tala --version` invocation, missing output, and malformed output such as `tala development`.
- [x] 3.8 Verify the renderer rejects a compatibility floor greater than the generated CLI version.

## 4. Validate and Document

- [x] 4.1 Update user-facing initialization or agent documentation to explain the compatibility metadata and refresh behavior.
- [x] 4.2 Run `cargo fmt --check` and `cargo test`.
- [x] 4.3 Run `openspec validate --all` and confirm the change is ready for implementation.
