## Implementation

- [x] Fix the client deadline so zero remains indefinite, including after transient transport failures.
- [x] Add the narrow `--only=inbox` wait mode and preserve its selection in continuation commands.
- [x] Advance and expose the durable event cursor correctly after filtered or returned events.
- [x] Suppress overdue events for inactive recipients while retaining once-per-message behavior; expose explicit recipient state on send.
- [x] Add focused CLI, daemon/store, and integration regression coverage, including isolated `TALA_HOME` where exercising binaries.
- [x] Run formatting, tests, clippy, sync specs, and archive the completed change.
