## Context

See `proposal.md` for the motivation. The CLI currently embeds the committed
OpenCode skill and command documents with `include_str!` and writes them during
`tala init` when the project contains `.opencode/`. The skill frontmatter has a
generic document `metadata.version`, while the command document has no version
metadata. The binary already exposes a stable `tala --version` value through
Clap, so the compatibility check can remain CLI-local and does not require a
daemon request.

## Goals / Non-Goals

**Goals:**

- Give both installed integration documents a machine-readable CLI minimum and
  generation version.
- Keep the skill document version independent from the CLI release version.
- Render the generating binary's package version when `tala init` installs the
  documents.
- Teach agents to compare the installed binary semantically and distinguish a
  hard incompatibility from an informational stale-document warning.
- Preserve compatibility with existing projects and older documents that lack
  the new metadata.

**Non-Goals:**

- No daemon wire-protocol or daemon restart behavior changes.
- No new top-level CLI command or machine-readable replacement for
  `tala --version`.
- No automatic scan or rewrite of every project on a user's machine.
- No change to the policy for overwriting existing project files; that should be
  handled by a separate safe-initialization change.

## Decisions

### Use explicit frontmatter fields

Add these fields to the skill metadata, while retaining the existing
`metadata.version` as the skill-content version:

```yaml
metadata:
  author: tala
  version: "3.1"
  tala_cli_min_version: "__TALA_CLI_MIN_VERSION__"
  tala_cli_generated_version: "__TALA_CLI_GENERATED_VERSION__"
```

The command document receives the two `tala_cli_*` fields in its frontmatter.
The minimum version is a maintained compatibility floor for the commands
documented by the files. For the current skill surface, the initial matrix is:

| CLI release | Relevant surface | Current skill compatibility |
|---|---|---|
| `0.25.0` | Pre-rename `chit` release | Incompatible |
| `0.25.1` | Core `tala` send/wait/history surface | Too old for intent and pending guidance |
| `0.27.3` | Intent, reply correlation, waiters, and pending-request guidance | Initial compatible floor for the current skill |
| `0.28.0` | Structured message parts and idempotent sends | Required only if those features are added to the skill |

This matrix is grounded in `CHANGELOG.md` entries for `0.27.3` and `0.28.0`,
and the tagged release READMEs for `0.25.0` and `0.25.1`. It SHALL be
re-audited when the skill command or flag surface changes. The initial
`tala_cli_min_version` is therefore `0.27.3` for the current committed skill.

The feature-to-release evidence used by the matrix is:

| Documented feature family | First release to verify | Evidence to retain in tests or release notes |
|---|---|---|
| `init`, `send`, `wait`, `history`, `list`, `listen`, `stream`, `check`, `close`, `status`, `stop`, and `wait --new-session` | `0.25.1` | Tagged `v0.25.1` README/help output |
| Session lifecycle, `use`, stdin/file input, and discovery guidance | `0.25.1` | `CHANGELOG.md` `0.25.1` entries and tagged help output |
| Intent, reply correlation, waiters, and `pending` | `0.27.3` | `CHANGELOG.md` `0.27.3` entry and tagged help output |
| Structured message parts and idempotent sends | `0.28.0` | `CHANGELOG.md` `0.28.0` entry; not part of the current committed skill floor |
Both placeholders are replaced by the binary at installation time: the
minimum comes from the maintained floor constant and the generated version
comes from `CARGO_PKG_VERSION`.

An explicit CLI-prefixed name avoids overloading the existing generic
`metadata.version` field. A protocol version field is intentionally omitted
from the skill metadata because protocol negotiation is a daemon concern and
does not describe CLI command availability.

Alternative considered: replace `metadata.version` with the CLI version.
Rejected because skill instructions can change independently of the binary and
existing consumers may already interpret that field as a document version.

### Render a placeholder at initialization time

The committed documents remain canonical templates. A pure renderer with the
shape `render_integration_document(template, min_version, generated_version)`
replaces both version placeholders and validates the rendered frontmatter. It
SHALL render and validate both complete documents before changing either
destination, and SHALL fail clearly if a required placeholder is missing or
appears more than once. Render failures SHALL leave both existing documents
untouched.

The renderer SHALL reject a minimum version greater than the generated version
because a binary must not generate a document that declares itself too old to
use. After successful rendering, each destination is written through a temporary
file and an atomic rename. This provides atomic replacement per document. A
filesystem failure after the first rename may leave the two destinations at
different versions; the command SHALL report that failure rather than claiming
the refresh completed. Full cross-file transactional replacement is not
required.

Alternative considered: manually update a literal CLI version in the committed
Markdown for every release. Rejected because it is easy for source and release
artifacts to drift, especially when a development checkout and installed
binary are at different versions.

### Make the skill check advisory and binary-authoritative

The skill instructions will tell agents to run `tala --version`, parse the
reported semantic version, and compare it with both metadata fields using
Semantic Versioning 2.0.0 precedence. Prerelease identifiers sort below their
corresponding release and build metadata does not affect precedence. Missing
or invalid values produce an unknown-compatibility warning. The expected
human-readable output is `tala <version>`; a failed command or output without a
valid version token is unknown compatibility.

- A binary below the minimum is an incompatibility warning; the agent should
  not use version-specific commands.
- A binary at or above the minimum but older than the generated version is a
  potential incompatibility; the agent must not assume the documented command
  surface exists and should upgrade or inspect `tala --help`.
- A binary newer than the generated version produces a stale-document warning;
  the agent should refresh the integration and verify every documented command
  and flag with `tala --help` before relying on them, because newer releases can
  remove as well as add commands.
- An exact generated-version match with a satisfied minimum is compatible.
- The local binary remains authoritative even if a project edits the metadata.

For compatibility state, versions that differ only in build metadata are
equivalent. A prerelease and its corresponding final release are not
equivalent.

No runtime semver dependency is needed for the CLI because the comparison is
performed by the agent. Tests will validate rendered CLI fields as
Semantic-Versioning values and exercise stable and prerelease precedence
fixtures in the documentation-checking helper.

Alternative considered: require exact equality between generated and installed
versions. Rejected because newer patch releases can remain
instruction-compatible, while an older binary than the generated document is
the higher-risk direction and is handled explicitly.

### Validate both templates and installed output

Documentation consistency tests will validate the committed templates for the
required keys and command guidance. Initialization tests will render the
templates through the binary, assert the generated version equals the package
version, and verify both installed documents contain the same compatibility
floor. A pure renderer test seam will accept arbitrary template strings so
missing and duplicated placeholders can be tested without changing
`include_str!` source files. Existing byte-for-byte tests will compare against a
rendered expected document rather than the placeholder template.

## Risks / Trade-offs

- [Risk] A stale skill can still document a command removed without a major
  version bump → [Mitigation] treat older-than-generated binaries as
  potentially incompatible and keep command-reference consistency tests in CI.
- [Risk] Project-local metadata can be edited or falsified → [Mitigation] treat
  `tala --version` as authoritative and use metadata only as guidance.
- [Risk] A source checkout may install documents whose generated version differs
  from a release binary → [Mitigation] render from `CARGO_PKG_VERSION` at
  runtime and expose the mismatch explicitly.
- [Risk] Older skill consumers may reject unknown frontmatter fields →
  [Mitigation] retain valid existing frontmatter and add ordinary metadata keys;
  unknown YAML metadata is non-breaking for the current OpenCode format.
- [Risk] `tala init` currently overwrites existing integration files, so a
  manual refresh can discard local edits → [Mitigation] this change performs no
  background refresh and documents the migration as an explicit user action;
  safe overwrite behavior remains a separate change.
- [Risk] Legacy documents have no metadata or compatibility guidance →
  [Mitigation] treat missing metadata as unknown compatibility when encountered
  by the new guidance and refresh them explicitly with `tala init`.

## Migration Plan

1. Add the metadata fields and renderer to the embedded templates.
2. Update `tala init` tests and documentation-consistency tests.
3. Release the change with the normal binary version.
4. Existing projects receive versioned documents the next time `tala init` is
   run; projects without `.opencode/` are unchanged. Because current init
   writes those files, users with customized integration documents must inspect
   or preserve them before refreshing. Legacy documents cannot self-report
   their age; only a refreshed version-aware document can provide the new
   guidance. The missing-metadata requirement applies to an agent using the
   new version-aware guidance while inspecting an older document; the older
   document itself cannot detect its own staleness.
5. Legacy documents without the new fields are considered unversioned until
   refreshed; no automatic repository-wide migration is attempted.
6. To roll back the binary, no data migration is required. Older binaries may
   overwrite the integration documents with their older templates, after which
   a newer `tala init` restores current metadata.
