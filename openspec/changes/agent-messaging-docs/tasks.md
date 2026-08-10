# Tasks — agent messaging docs (#65)

## 1. README

- [x] 1.1 Add "Agent handshake" canonical end-to-end example (init → session
      create --name → send --intent req --wait → wait --new-session →
      reply --reply-to → pending → close)
- [x] 1.2 Extend "Sending Messages" with the input-method decision table
      (argv / piped heredoc / --stdin / --message-file / --part)
- [x] 1.3 Add intent precedence paragraph (explicit > reply-to implies reply
      > wait implies req; reply-to+wait = reply+expect-reply; re-ask pattern)
- [x] 1.4 Refresh command table: add `pending`, `use`, `session` group,
      intent flags

## 2. SKILL.md (agent-facing)

- [x] 2.1 Add intent precedence + re-ask pattern to Intent Protocol section
- [x] 2.2 Input-method guidance already matches README (heredoc-first,
      --stdin/--message-file for special characters)

## 3. Validation

- [x] 3.1 README examples verified against v0.28.0 live daemon (session
      create id capture, wait --new-session bare-id stdout, reply-to,
      pending, close — all exercised in cycle-18 evals and live probes)
- [x] 3.2 No stale command names left in README (grep check vs `tala --help`:
      recap/observe/start/follow absent)
