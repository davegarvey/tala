# Interrupted blocking send

Reproduces #93 and #97: the daemon stays alive but stops answering while an agent is blocked in `send --wait`. Use one scratch directory and an isolated Tala home. Never pause a shared daemon.

## Setup

```bash
export TALA_HOME="$(mktemp -d)/home"
"$TALA_BIN" register --name=a --tool=fixture   # keep A's selection
"$TALA_BIN" register --name=b --tool=fixture   # keep B's selection
```

## Steps

1. As A, start `send --to=<B id> --wait --timeout=0 "question"` in the background.
2. Pause the daemon: `kill -STOP "$(jq .pid "$TALA_HOME/daemon.json")"`.
3. Leave it paused long enough for the waiting client to give up (300 seconds against 0.34.0), then `kill -CONT` it.
4. Separately, repeat with a 20-second pause, and have B reply to the request after resuming the daemon.
5. Record A's exit code, stderr, `pending --direction=outgoing` and `daemon.log`.

## Expected

- The client never starts a second daemon while the first is alive; `daemon.log` has no "already owns this Tala home" line.
- A short pause is absorbed: A receives B's reply and exits 0.
- A pause that outlasts the deadline exits 4, not 1, with `stored` holding the request ID and `next` resuming the wait without resending.
- Exactly one outgoing request is stored.

## Baseline (0.34.0, 2026-10-12)

- 15-second pause, 60-second deadline: the client spawned a second daemon, which failed on the home lock; the wait ended with exit 3 because the deadline expired while the client was blocked in its restart loop.
- Indefinite wait, pause held until the client gave up: after 267 seconds the send exited 1 with `DAEMON_START_FAILED`, although request 1 was stored. `daemon.log` showed the spawned second daemon failing on the home lock. A caller reading exit 1 as "not sent" would resend.
