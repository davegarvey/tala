# Design

## Context

`Context.project` and `Context.worktree` currently describe registration and also get copied onto outgoing messages. Agent selection and board routing are already independent of the current directory.

## Goals / Non-Goals

**Goals:** Let recipients locate the sender's current checkout while keeping identity and board routing stable.

**Non-Goals:** Automatically switch identity, move board membership, or replace registered project metadata.

## Decisions

Add an optional canonical `working_directory` field to context. At registration it records that command's directory; when constructing outgoing context, refresh only this field from the current process directory while preserving registered project/worktree, branch, and revision. This explicit field avoids changing the meaning of existing project fields. The field is additive and optional so older stored rows and callers remain readable.

## Risks / Trade-offs

The directory may not be accessible to a peer on another machine. It remains a local hint; messages still need to carry the relevant text or data for remote collaboration. Canonicalization errors should follow normal command error handling rather than emit a misleading path.
