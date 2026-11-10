# Spec Delta

## MODIFIED Requirements

### Requirement: Network configuration commands
`tala enable-network` SHALL store the bind address and a private access token in the home, and `tala disable-network` SHALL remove that configuration. When no daemon is running, `enable-network` SHALL start one so the listener outcome can be reported. Otherwise changes SHALL take effect on the next daemon start, and both commands SHALL report when a restart is required. Suggested connection URLs SHALL list private LAN addresses before other addresses.

#### Scenario: Enable network access
- **WHEN** a user runs `tala enable-network --bind=192.168.1.20:7676`
- **THEN** the home stores the bind address and a private access token, and the output includes the token and a `connect-remote` command to run on the other machine

#### Scenario: Token is stable unless rotated
- **WHEN** `enable-network` runs again without `--rotate-token`
- **THEN** the existing token is kept, and with `--rotate-token` a new token replaces it

#### Scenario: Enable network with no running daemon
- **WHEN** a user runs `enable-network` while no daemon is running
- **THEN** Tala starts the daemon, reports the bound address or bind error, and reports no restart required

#### Scenario: Enable network while a daemon runs
- **WHEN** a user changes network configuration while a daemon without that configuration is running
- **THEN** the command reports `restart_required` with `next` of `tala stop`
