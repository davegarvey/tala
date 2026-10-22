# Design

## Context

`send --wait` stores a request, then calls the correlated inbox waiter. A normal deadline currently returns an `ok` timeout envelope with the sent message snapshot, while transport failures return structured errors with a `next` command.

## Goals / Non-Goals

**Goals:** Make the existing stored request resumable directly from timeout output.

**Non-Goals:** Change exit codes, message storage, retry behavior, or inbox wait semantics.

## Decisions

Add a `next` field to the timeout data for `send --wait`, formed from the known message ID and original timeout. Keep the timeout result successful at the transport layer and retain exit code 3; the caller still distinguishes task timeout through `timed_out` and the exit code.

Do not change general `inbox --wait` or `board --wait` timeout output, since they do not follow a stored outgoing request.

## Risks / Trade-offs

The command is a string and must be shell-quoted safely if IDs or values ever cease to be numeric. Current message IDs and timeout are numeric. Human rendering should expose the same recovery action as JSON.
