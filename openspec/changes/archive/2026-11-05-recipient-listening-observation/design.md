# Design

The store already resolves the recipient and has a `listening_until` lease updated by `inbox --wait`. At message creation, compute `active && listening_until > now()` for the resolved direct recipient and save it in a nullable `recipient_listening` column on the message row. Direct replies use the original message's sender as their recipient through the existing routing path. Board messages have no direct recipient and therefore omit the field.

When an idempotency-key retry returns an existing direct message, include the saved boolean rather than checking current presence again. Request-ID replay already returns the complete cached JSON response. For rows created before the migration, freeze the first observation on their first key-based replay.

Add a transactional database migration from schema version 1 to 2. The wire protocol does not change: this is an additive JSON result field and an internal database column. Human output describes the point-in-time observation and says an unlistened message remains queued; it does not promise a response.
