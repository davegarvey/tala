# Design

## Context

The `agents` table owns identity and presence fields; `agent()` is the shared projection used by registration, `whoami`, and `agents`. Authenticated operations normally refresh `last_seen`, while request/response caching already makes registration and rename retries idempotent. SQLite schema version 3 contains the `supersedes` migration; the wire protocol is version 6.

## Goals / Non-Goals

**Goals:** Store nullable public purpose and a separate nullable purpose-update timestamp; preserve identity/presence/history when purpose changes; keep displayed text safe; and reject old daemon protocols before the new operation can mutate storage.

**Non-Goals:** Purpose will not affect agent naming, routing, visibility, authorization, or project membership. This change will not rename identities or add private purpose metadata.

## Decisions

- Add optional `purpose` and `purpose_updated_at` fields to the public `Agent` projection. Keep the timestamp nullable and in the same Unix-seconds format as other stored times. Registering a non-empty purpose records the registration time; omitted or blank purpose leaves both fields null. Setting or clearing purpose records the current time, even if the requested value equals the stored value.
- Normalize at the daemon boundary: reject C0/C1 controls and Unicode `Bidi_Control` code points before trimming, trim surrounding Unicode whitespace, treat empty text as absent, then enforce the 160-scalar limit. The bidi set is U+061C, U+200E–U+200F, U+202A–U+202E, and U+2066–U+2069. Applying the control check before trim ensures a newline-only input is rejected instead of silently becoming empty.
- Represent `update-purpose --clear` and an empty normalized purpose as `UpdatePurpose { purpose: None }`. Require exactly one of `--purpose` and `--clear` at the CLI. Add the operation to cached mutations so a retried request ID returns the original response and timestamp.
- Purpose edits do not refresh `last_seen` or `active`; special-case this operation in the common authenticated-operation prelude. `purpose_updated_at` is the only agent field changed by a purpose edit.
- Append two nullable columns in a transaction guarded by schema version `< 4`; do not add them to the base `CREATE TABLE agents` statement. Fresh version-0 databases run the existing v3 migration followed by v4 exactly once, while existing v3 rows receive null values. Bump the protocol constant to 7 so v6 clients and daemons cannot silently ignore the new fields or operation.
- JSON returns the full stored text and explicit null fields. Human purpose output escapes control and bidi formatting characters defensively, then truncates at 80 Unicode scalars and marks truncation. Render `whoami --human` explicitly; keep peer-list purpose optional when unset.

## Risks / Trade-offs

- A purpose is visible to every agent in the local Tala home, including peers in other projects → Explain visibility in command help and OpenSpec, and do not treat it as trusted instructions or private data.
- Character-based validation can be confused with UTF-8 byte limits → Count Rust `char`s after normalization and test multibyte text at both limit boundaries.
- Migration failure could leave a partially upgraded home → Run both `ALTER TABLE` statements and `user_version=4` in one transaction; test v3 migration and a fresh v0 database.
- Human output can be spoofed by corrupted or manually edited storage even when new input validation is strict → Escape controls and bidi formatting at render time as well.

## Migration Plan

On first open of a v3 database, transactionally add nullable `purpose` and `purpose_updated_at` columns and set `PRAGMA user_version=4`. Existing rows remain null and identity/message tables are untouched. Fresh databases first pass through existing v1→v2→v3 migrations and then the same v3→v4 step. Update the daemon protocol constant to 7; mismatched v6 daemons are already rejected before mutation. Rollback is by restoring the database backup and using a matching pre-upgrade Tala binary; do not delete state.
