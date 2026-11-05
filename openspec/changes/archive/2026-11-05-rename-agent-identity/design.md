# Design

The CLI sends a new authenticated `rename` operation containing only the requested name. The daemon derives the target ID from the verified credential, rejects a blank name, and updates only `agents.name`. It returns the updated agent, previous name, and whether the value changed. An unchanged name is a successful no-op.

Names remain non-unique. Existing `resolve` behavior continues to prefer an exact public ID and reject a name matching multiple agents. The existing request-result table caches the rename response by request ID, making an identical retry safe and rejecting reuse of that ID for a different operation.

This adds a wire operation, so `PROTOCOL_VERSION` advances from 4 to 5. No database migration is required. The guide distinguishes a rename (which requires the current handle) from lost-handle recovery and from registering a genuinely new instance.

Risks considered:

- A stale credential must never rename another identity: the operation carries no target ID, and the store uses only the authenticated agent ID.
- A duplicate name can make addressing by name ambiguous: this is existing behavior and exact public IDs remain available.
- Users may mistake rename for credential recovery: CLI help and the guide say the selected identity must already be available.
