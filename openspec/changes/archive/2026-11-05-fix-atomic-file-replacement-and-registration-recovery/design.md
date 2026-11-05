# Design

## Context

See proposal.md - Why. File replacement is shared by daemon metadata and generated instructions. Registration credentials are returned by the daemon, then persisted locally as a private handle mapping.

## Goals / Non-Goals

**Goals:**
- Keep staged replacement atomic on supported platforms when the destination already exists.
- Retain a private path to the daemon-issued credential when handle persistence fails.

**Non-Goals:**
- Change the wire protocol, identity model, or normal handle format.
- Recover credentials after all writes to the Tala home fail.

## Decisions

- Route staged replacements through one helper. On Unix, use `rename`; on Windows, use `MoveFileExW` with replacement and write-through flags. This preserves same-volume staging and avoids a remove-then-rename window. Additive dependencies are unnecessary.
- Use the existing ID-keyed reissue file location as the fallback only when writing the new handle fails. If that fallback succeeds, return an error that states registration was committed and supplies `tala register --reissue=<agent-id>`. Reissue already validates the credential, writes a normal handle, and removes the fallback file.
- Keep fallback credentials private with the same 0700 directory and 0600 file permissions used for handles.

## Risks / Trade-offs

- [Both the handle write and recovery fallback write can fail, for example when storage is full] → Report the agent ID and both storage errors; no credential can be preserved without a successful write.
- [Windows replacement depends on the destination not being held open incompatibly] → Preserve the OS error and leave the staged file cleanup behavior intact.

## Migration Plan

No data migration is needed. Existing handle files and pre-handle reissue files keep their current format. The fallback file is removed by the existing successful reissue flow.
