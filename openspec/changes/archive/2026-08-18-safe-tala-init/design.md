## Context

The previous skill-versioning change added a renderer and atomic per-file
writes, but `cmd_init` still creates `.tala/config.json` eagerly and then
always writes both rendered OpenCode documents. The CLI has no init-specific
flags or action report. See `proposal.md` for the motivation and
`openspec/changes/archive/2026-08-18-versioned-tala-skills/` for the existing
rendering contract.

## Goals / Non-Goals

**Goals:**

- Make repeated `tala init` safe without introducing a second setup command.
- Give agents and scripts a deterministic dry-run and JSON action report.
- Require an explicit overwrite decision for locally different integration
  files.
- Keep config identity preservation and `.opencode/` auto-detection intact.
- Make Git-ignore setup explicit and repository-root aware.

**Non-Goals:**

- No three-way merge of user-edited Markdown.
- No automatic modification of `.gitignore` or any other file without an
  explicit flag.
- No automatic `.DS_Store` or unrelated global ignore policy.
- No rename of `tala init` and no daemon or wire-protocol changes.

## Decisions

### Use one idempotent init command with explicit controls

Extend the existing `Init` command with `--dry-run`, `--force`, `--gitignore`,
and `--json`. The positional name remains supported. `--force` affects only
different generated integration files; it never changes an existing
`.tala/config.json`. `--dry-run` takes precedence over writes even when
`--force` or `--gitignore` is supplied, allowing users to preview a destructive
refresh.

Alternative considered: add `tala setup` or `tala refresh`. Rejected because
`init` is already documented and is naturally repeatable; flags make the
destructive choice visible without expanding the command surface.

### Build an action plan before writing

Initialization will first render the current integration documents, inspect
the config and integration destinations, and produce an in-memory plan. Each
path receives one action:

- `created` when missing and writing is enabled
- `unchanged` when the existing content matches
- `skipped` when content differs and `--force` is absent
- `overwritten` when content differs and `--force` is present
- `would_create`, `would_unchanged`, `would_skip`, or `would_overwrite` in
  dry-run mode

The config action is `created`, `unchanged`, `would_create`, or
`would_unchanged`. The plan always treats an existing config as immutable. A
different integration file is compared byte-for-byte against the rendered
content; no attempt is made to infer or merge user edits.

All rendering and validation occurs before the plan is applied. Applying a
non-dry plan writes only actions that require writes, using the existing atomic
per-file writer. A failure is reported and does not claim that skipped or
unattempted actions completed.

### Keep human and JSON output separate

Human-readable action summaries go to stdout for normal init output, while
warnings about skipped or unavailable actions go to stderr. `--json` emits one
JSON object on stdout and keeps diagnostics on stderr. The object has this
shape:

```json
{
  "config": {"path": ".tala/config.json", "action": "unchanged"},
  "files": [
    {"path": ".opencode/skills/tala/SKILL.md", "action": "skipped"},
    {"path": ".opencode/commands/tala.md", "action": "unchanged"}
  ],
  "gitignore": {"path": null, "action": "not_requested"},
  "warnings": ["..."],
  "dry_run": false
}
```

The `files` list is empty when `.opencode/` is absent. Git-ignore actions use
`not_requested`, `added`, `present`, `would_add`, or `unavailable`.

### Make Git-ignore setup opt-in and root-aware

When `--gitignore` is supplied, invoke Git to find the repository root. Read
the root `.gitignore` and recognize an existing `.tala/` ignore rule by an
exact non-comment line matching `/.tala/`, `.tala/`, or `**/.tala/`. If no
matching rule exists, append `/.tala/` with correct newline handling. Create a
missing root `.gitignore` only under the explicit flag. Outside a Git
repository, report `unavailable` and warn without creating a file.

This intentionally treats the entire `.tala/` directory as local state,
matching the repository's current policy. No default init path edits ignore
rules.

### Keep generated document ownership conservative

An exact content match is the only automatic proof that an existing integration
file is safe to leave untouched. `--force` is the explicit user acknowledgement
that a different file may be replaced. The command will not use metadata alone
to decide that a manually edited file is safe to overwrite.

## Risks / Trade-offs

- [Risk] Users must pass `--force` to refresh older generated files →
  [Mitigation] default output names each skipped file and the README documents
  the explicit refresh path.
- [Risk] A filesystem failure after one per-file rename can leave the two
  integration files at different versions → [Mitigation] render all files
  before writes, use atomic replacement per file, and report the failed action.
- [Risk] Git-ignore detection may miss unusual generated or global ignore rules
  → [Mitigation] only promise exact repository-root pattern detection and leave
  unrelated ignore configuration untouched.
- [Risk] JSON action schemas become automation contracts → [Mitigation] test
  every action family and keep field names stable and additive.

## Migration Plan

1. Release the safer init behavior with the existing versioned integration
   renderer.
2. Existing projects continue to use their config; repeated init no longer
   rewrites different integration files by default.
3. Users who intentionally want refreshed generated docs run `tala init
   --force` after reviewing the reported plan.
4. Users who want local Tala state ignored run `tala init --gitignore`; no
   migration edits `.gitignore` automatically.
5. Rollback requires no data migration. Older binaries may retain their prior
   overwrite behavior, so users should use the newer binary when refreshing
   existing integrations.
