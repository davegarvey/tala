## MODIFIED Requirements

### Requirement: Replies and obligations
The system SHALL route reply automatically to the original sender within its thread and permit replies only by the addressed recipient. Requests SHALL remain pending after receipt until explicitly replied to, completed by their recipient with a linked answer, cancelled by their sender, or superseded by a later direct message from that sender to the same recipient in the same thread. A follow-up request SHALL not settle another request. A recipient SHALL NOT reply to a superseded request; the error SHALL identify the superseding message. An explicit acceptance SHALL record a commitment separately from reply, and reply SHALL complete the request whether or not it was accepted. A recipient MAY complete an active request by linking a newer ordinary direct message sent to the request sender; the answer may be in another thread.

#### Scenario: Delivery does not imply acceptance
- **WHEN** a recipient fetches a request without accepting it
- **THEN** it remains pending and unaccepted

#### Scenario: Reply completes an unaccepted request
- **WHEN** the addressed recipient replies before accepting
- **THEN** the request is answered and no acceptance deadline alert is emitted afterward

#### Scenario: Recipient links a separately sent answer
- **WHEN** the addressed recipient completes an active request with a newer ordinary direct message addressed to the request sender
- **THEN** the request records that message as its answer and is no longer pending
- **AND** the answer remains in its original thread and completion is visible in durable history

#### Scenario: Invalid linked answer is rejected
- **WHEN** the linked message is not an ordinary direct message from the addressed recipient to the request sender, or its ID is not newer than the request
- **THEN** completion fails without changing either message

#### Scenario: One answer completes multiple requests
- **WHEN** the recipient links one valid answer to multiple requests from the same sender
- **THEN** each request is completed and retains the shared answer reference

#### Scenario: Accepted request is superseded
- **WHEN** a sender supersedes a request after the recipient accepted it
- **THEN** the old request is no longer pending and the sender receives a durable superseded event

#### Scenario: Receipt is not an answer
- **WHEN** a recipient consumes a request
- **THEN** delivery becomes received but the request remains pending

#### Scenario: Superseding a request clears its obligation
- **WHEN** a sender posts a direct message that supersedes its unanswered request to the same recipient in the same thread
- **THEN** the older request is no longer pending
- **AND** both messages remain visible in the thread with their relationship

#### Scenario: Reply to superseded request is rejected
- **WHEN** a recipient replies to a superseded request
- **THEN** Tala returns an actionable error naming the successor message ID
- **AND** no reply message is stored
