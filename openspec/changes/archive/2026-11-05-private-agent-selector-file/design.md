# Design

Selector files are plain UTF-8 text containing one `th_` handle and an optional final LF or CRLF. Reads are size-bounded, reject malformed content, and validate the opened file rather than trusting a path lookup.

On Unix, open with `O_NOFOLLOW`, require a regular file owned by the effective user, and reject any group/other permission bits. Create the destination itself with `create_new` and mode `0600`; require an existing directory owned by the user that is not group/world writable. Never change parent permissions.

On Windows, create with `CreateFileW(CREATE_NEW)` and a protected DACL granting full access only to the current user and LocalSystem. Open with `FILE_FLAG_OPEN_REPARSE_POINT`; inspect file attributes, owner SID, protected DACL, ACE type, SID and mask before reading. Any inspection failure or unexpected ACE fails closed.

CLI source resolution is centralized. An explicit `--agent` or `--agent-file` overrides environment selectors; the two explicit options conflict. If neither is explicit, exactly one of `TALA_AGENT` and `TALA_AGENT_FILE` may be set. Present-but-empty values are errors. A successful `--save-handle-to` result includes the selector path and non-secret `--agent-file` instruction, not the handle. If the external write fails after registration and internal handle storage succeed, return the ordinary handle once with an explicit save warning.

No wire-protocol or database change is required.
