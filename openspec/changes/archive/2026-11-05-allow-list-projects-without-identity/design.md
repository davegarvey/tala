# Design

## Context

See proposal.md - Why. The CLI already treats `agents` as unauthenticated discovery, while project listing requires a credential at both the CLI and store boundaries. Authenticated RPC responses receive ambient attention after the operation; unauthenticated ones do not.

## Goals / Non-Goals

**Goals:**
- Make project listing usable before an agent registers.
- Preserve ambient attention for callers that do provide a valid identity.

**Non-Goals:**
- Change access to project lifecycle operations or other identity-scoped commands.
- Change project visibility, filtering, pagination, or wire protocol.

## Decisions

- Make the CLI credential optional for `ListProjects`, matching `Agents`.
- Remove the store's identity requirement for `ListProjects`. The shared RPC layer will continue to authenticate and validate a supplied credential; valid credentials still become the operation identity and receive ambient attention.
- Document pre-registration discovery in command help and the agent guide. No per-project authorization is introduced because project listing is already server-wide and `agents` is unauthenticated.

## Risks / Trade-offs

- [Unauthenticated callers can see project paths and owner metadata] → This matches existing unauthenticated `agents` discovery, which exposes each registered agent's project path, and all Tala data is already scoped to a local Tala home.
