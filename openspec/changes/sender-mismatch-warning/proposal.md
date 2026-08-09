# Warn when `--sender` differs from the configured agent identity (B004 interim)

## Why

`tala send <session> "msg" --sender <any-name>` lets any local user or agent emit
messages as **any** identity. `history`, `agents` and `discover` all trust the
`sender` field verbatim, so a spoofed message is indistinguishable from a real
one to every reader (backlog B004, reconfirmed cycles 01–15).

This is a genuine product decision — the end state is either *restrict*
(`--sender` may only be the configured agent identity) or *authenticate*
(daemon records a separate, unforgeable identity alongside the display name).
That decision belongs to the maintainer, and changing either direction is
breaking.

Meanwhile, the failure mode today is **silent**: nothing tells the sender (or an
operator watching) that the identity being asserted is not the project's own.
The minimal honest improvement is to make impersonation *visible* at the point
of emission, without pre-empting the design call:

- A sender running `--sender spoofed-agent` in a project configured as `alpha`
  gets an immediate warning that recipients will see a different identity.
- `--json` automation gets a structured signal (`sender_mismatch`,
  `configured_sender`) instead of only prose.

## What Changes

CLI-side only (src/cli.rs). The daemon is untouched — it cannot authenticate a
local sender, so the warning belongs where the identity is known: the sending
process, which knows both the override (`--sender`) and the project identity
(`.tala/config.json` `name`, falling back to the directory name).

- In `cmd_send`, after content resolution, compute the effective sender with and
  without the override. If an override is present **and** differs from the
  configured identity:
  - print a `Warning:` line to stderr in all modes (including `--json` and
    `--quiet` — this is an integrity signal, not cosmetic output);
  - pass the mismatch through to the JSON success response so parsers see
    `"sender_mismatch": true` and `"configured_sender": "<name>"`.
- When `--sender` matches the configured identity (or is absent), nothing
  changes: no warning, no extra fields.

Behavior is deliberately non-blocking: sending still proceeds. Restriction or
authentication is left to the open design call.

## Capabilities

- Operators and agents can detect that a message they just sent (or an automated
  pipeline sent) asserts a non-local identity, in both human and machine-readable
  form.
- Existing scripts that pass `--sender` for legitimate cross-identity sends keep
  working; the new warning does not alter exit codes or message delivery.
- The daemon API, message schema, and wire format are unchanged.

## Impact

- **CLI**: one new stderr warning + two optional JSON fields on the send ack.
- **Tests**: existing e2e tests that use `--sender` in unnamed project dirs will
  now see the warning on stderr — no test asserts empty stderr, so none break;
  new tests assert the warning fires on mismatch and is absent on match.
- **Docs**: `--sender` help text gains "(warns if it differs from the configured
  agent name)".
- **Migration**: none. Behavior is additive.

## Non-goals

- Restricting or blocking `--sender` (design call, maintainer decision).
- Daemon-side authentication of senders (design call).
- Detecting impersonation of *other* agents by readers (follow-up).
