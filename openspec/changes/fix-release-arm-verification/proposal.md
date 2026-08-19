## Why

The first release after the installation hardening change exposed that an
ARM64 Linux binary cannot be executed directly on the x86_64 GitHub runner.
The new verification step therefore fails before packaging even though the
cross-built artifact is valid.

## What Changes

- Run ARM64 Linux version checks through an ARM64-compatible container runtime.
- Keep native execution checks for targets that match their runner architecture.
- Preserve archive, checksum, and extraction verification for every target.
- Do not change release asset names, installation metadata, or runtime code.

## Capabilities

### New Capabilities

None. This is a release-workflow-only correction; `.openspec.yaml` sets
`skip_specs: true`.

### Modified Capabilities

None.

## Impact

- Affected file: `.github/workflows/release.yml`.
- Affected release jobs: ARM64 Linux artifact verification.
- No user-facing CLI, crate, protocol, or data changes.
