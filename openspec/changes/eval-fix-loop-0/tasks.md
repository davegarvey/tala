## 1. Rename watch to stream

- [x] 1.1 Change `Commands::Watch` primary name from `"watch"` to `"stream"`, add `alias = "watch"`, add deprecation handler that warns and delegates
- [x] 1.2 Update `tala --help` long_about text to reference `tala stream` instead of `tala watch`

## 2. Stream non-empty on timeout

- [x] 2.1 Add message counter to `cmd_watch`; after the SSE loop, emit `[no messages received]` (text) or `[]` (JSON) if zero messages arrived

## 3. Send --wait progress heartbeat

- [x] 3.1 Split the single long-poll GET in `cmd_send` wait section into multiple shorter polls with a spinner dot printed between retries

## 4. Observe deprecation visibility

- [x] 4.1 Bump `deprecation_warning("observe", "listen")` to use a stronger format (e.g., `"error:"` prefix or emphasized message)

## 5. Listen help text clarity

- [x] 5.1 Update `Commands::Listen` doc comment to include "all sessions" in the description
- [x] 5.2 Update `tala --help` long_about to clarify listen (all sessions) vs stream (one session)

## 6. Send missing stdin error hint

- [x] 6.1 Add mention of `--stdin` to the error messages in `cmd_send` when no message source is found
