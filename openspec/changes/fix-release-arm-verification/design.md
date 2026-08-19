## Context

The release matrix cross-builds `aarch64-unknown-linux-musl` on an x86_64
Ubuntu runner. The verifier currently invokes that binary directly, which
produces an exec-format failure. The artifact is statically linked for musl,
so it can be executed in an ARM64 Alpine container using the runner's platform
emulation.

## Goals / Non-Goals

**Goals:**

- Verify the ARM64 Linux binary and extracted archive report the release version.
- Keep the verification behavior for native macOS, Windows, and x86_64 Linux
  targets unchanged.
- Make the release workflow pass on the existing matrix without adding a new
  runner class.

**Non-Goals:**

- No change to cross compilation or release asset naming.
- No additional runtime dependency in the Tala binary.
- No attempt to emulate unsupported release targets.

## Decisions

Use `docker run --platform linux/arm64 alpine:3.22` for the ARM64 Linux
version checks. The release binary is a musl executable and can run directly
in that image. Mount the build or extraction directory read-only, keeping the
verification isolated from the runner filesystem.

Alternative considered: skip execution for cross-built targets and only inspect
the ELF architecture. Rejected because it would not verify the embedded release
version or actual startup behavior.

Alternative considered: add an ARM64 GitHub runner. Rejected because the
existing matrix already uses Docker-based cross compilation and platform
emulation is sufficient for this static verification.

## Risks / Trade-offs

- [Risk] Docker platform emulation or the Alpine image could become unavailable
  → [Mitigation] keep archive and checksum checks independent, and fail clearly
  at the ARM verification step rather than silently skipping it.
- [Risk] The Alpine tag can change over time → [Mitigation] use the minor
  version tag and keep the command limited to release verification.

## Migration Plan

1. Update the ARM64 Linux verification conditions in the release workflow.
2. Run the release workflow on the next version bump and confirm all matrix
   jobs complete.
3. If the emulation path is unavailable, revert only the verifier change and
   retain the artifact packaging checks while choosing a native runner.
