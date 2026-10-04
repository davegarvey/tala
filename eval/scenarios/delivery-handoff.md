# Delivery and handoff

Register A, B and successor C in scratch projects with a shared isolated Tala home. Give each docs/agent-guide.md.

A: Send B a request with a short wait timeout, then inspect delivery and pending status. Do not cancel it.
B: Peek, read history, then consume the request. Report that receipt is separate from answering. Hand off the thread to C with a concise summary, receive C's answer, then reply to A after A's wait has expired.
C: Receive the handoff, inspect the granted thread and reply to B.
A: Retrieve the late reply and inspect settled pending work.

Expected: peek/history do not acknowledge, consumption does, timeout leaves the request stored, C sees only explicitly granted direct history, late reply is retrievable, and no agent treats another's message as user authorization. Record CLI evidence and agent feedback. Stop the daemon only after evidence collection.
