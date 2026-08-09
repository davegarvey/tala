# Make `history --limit N` return the newest N messages (B016)

## Why

Backlog B016 (reconfirmed fresh on main caf8718 in cycle-11): `tala history
<session> --limit N` returns the **oldest** N messages, not the newest.
Repro on a 4-message session (`sess_u4edb`):

```
$ tala history sess_u4edb --limit 2
session: sess_u4edb  |  created: 2026-08-07 08:24:30  |  closed: false
cursor: 2

[1] alpha (08:24:34):
    beta, cycle-11 handshake: what is the capital of cyc-11?

[2] beta (08:25:45):
    reply: Reykjavik (cycle-11 handshake OK)
```

A user asking for `--limit 2` wants the **last two messages** ("what happened
most recently") — ids 3,4 — not the first two. The header `cursor: 2` (the max
id shown) then reads like "next-to-read", doubling the confusion: it is
literally the *oldest* boundary, not the frontier.

Root cause: `store.rs get_messages_filtered` (src/store.rs:177-178) applies
`result.into_iter().take(limit)` on messages in ascending-id order, i.e. it
takes the FIRST N of the filtered set. The server applies `limit` in the recap
path (src/api.rs:529) and the wait path (src/api.rs:233/283/308) — all three
suffer the same inversion for any session whose matching messages exceed the
limit.

## What Changes

- `get_messages_filtered` (src/store.rs): when a limit is set, return the
  **newest** N messages — skip `len - limit` and keep the tail — while
  preserving ascending-id display order in the result. `take` becomes
  `skip(len.saturating_sub(limit))`.
- The recap/history header `cursor: <max id shown>` then reports the highest id
  actually returned (the newest boundary), which is the honest "you have seen
  up to here" value and matches the cursor written by `cmd_recap`.
- `wait --limit N` (api.rs wait paths) also returns the newest N of the
  matching set — consistent with "limit = cap the messages I get" and strictly
  more useful for automation (the latest messages, not the first burst).
- `--limit 0` stays unlimited; `--since`/`--from` filtering unchanged.

## Capabilities

- `tala history <sess> --limit 2` on a 4-msg session prints ids 3,4 (newest),
  header `cursor: 4`.
- `tala history <sess> --limit 2 --json` returns the 2 newest messages with
  `cursor` = max id returned.
- `tala wait <sess> --limit 2` returns the 2 newest matching messages.
- `--limit 0` / no limit still returns everything matching since/from.

## Impact

- Behavior change to `--limit` semantics on `history` and `wait` — this is the
  fix, matching documented intent ("Maximum number of messages to show").
- Existing e2e tests `test_recap_limit_cap`, `test_recap_limit_zero_is_unlimited`,
  `test_wait_limit_cap` only assert count — they stay green; extend them to
  assert WHICH messages are returned (newest, not oldest).
- No API shape change; no cursor-model dependency (independent of unmerged
  PR #46 per-session cursors — the ordering fix is orthogonal and lands on a
  clean main base).
