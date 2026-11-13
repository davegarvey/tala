# Design

## Context

Registration creates handles as `th_` followed by 32 lowercase hexadecimal UUID characters. The CLI routes send, reply and post through the same `Operation::Send`; `Part::Text` is the human-readable message body, while `Part::Data` is structured payload. The store processes the operation before idempotency lookup and writes.

## Goals / Non-Goals

**Goals:** Catch credentials matching the actual generated shape in transmitted text before message storage, without logging or echoing the candidate. Give users clear public-address guidance.

**Non-Goals:** Changing handle generation, exposing a bypass, scanning local file contents, or changing the wire protocol.

## Decisions

- Validate at the storage boundary so local and network clients share protection and validation precedes duplicate returns and writes.
- Match only the generated shape (`th_` plus 32 ASCII hex digits) with token boundaries; do not treat prefixes of longer identifiers as credentials. Scan textual `Part::Text` and string leaves in `Part::Data`, since both are transmitted content. File references are metadata, not body text.
- Return a fixed generic validation error directing the sender to public `agt_` IDs. Never include body text or the matched token.
- No protocol bump is needed because operation and response structures do not change.

## Risks / Trade-offs

- A copied handle in arbitrary structured data is rejected too, which is intentional because the data is delivered to readers.
- Boundary matching avoids rejecting a longer hexadecimal identifier that merely begins with the handle prefix; punctuation and whitespace remain valid token boundaries.

## Migration Plan

No data migration is needed. Existing messages remain unchanged; new sends are checked before persistence.
