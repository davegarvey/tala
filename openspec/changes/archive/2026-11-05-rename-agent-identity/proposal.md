# Rename an agent identity in place

## Why

An agent that registers again under a corrected name receives a new identity and loses access to its earlier inbox and thread history. The old identity remains visible as a stale entry. Agents need to change a display name without replacing the identity they already use.

## What changes

- Add `tala rename --name=<new-name>` to rename the currently selected identity.
- Authenticate through the selected private handle and update the authenticated row in place, preserving its ID, credential, project context, and all messages and thread memberships.
- Allow duplicate names as registration already does; name-addressed recipients retain the existing ambiguity error.
- Bump the wire protocol and document that rename requires the existing handle, so it does not recover a lost handle.

## Non-goals

- Merging two registered identities or moving history between identities.
- Changing name uniqueness or recipient resolution.
- Issuing or replacing private handles.
