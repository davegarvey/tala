## Context

See `proposal.md` for the motivation and `specs/` for the observable
requirements. Tala already renders version metadata into its OpenCode
integration documents during `tala init`, and the CLI already performs a
pre-command compatibility check for daemon protocol versions. The remaining
gap is that repository-local skill files can outlive the binary that generated
them, while unknown subcommands are rejected before the normal command runner
can provide guidance.

## Goals / Non-Goals

**Goals:**

- Detect the nearest repository-local Tala integration without contacting the
  daemon or scanning unrelated projects.
- Use the same selected project root for normal initialization, checking, and
  refresh, including invocations from nested directories.
- Treat the installed binary and its public Clap command surface as the source
  of truth.
- Make stale, unversioned, and missing compatibility metadata actionable while
  preserving normal command behavior and machine-readable stdout.
- Make integration refresh explicit and prevent an ordinary `tala init` from
  overwriting existing agent instructions.
- Make the reduced public command surface explicit in the CLI specification and
  documentation validation.
- Add coverage for valid commands, JSON output, initialization, and unknown
  subcommands.

**Non-Goals:**

- Restoring removed commands or adding aliases for commands outside the chosen
  public surface.
- Inspecting global or cached agent instructions that Tala cannot locate from
  the current project.
- Parsing all Markdown prose at runtime to prove semantic documentation
  correctness. Source-template documentation checks remain the appropriate
  place for exhaustive command-reference validation.
- Changing the daemon protocol, session persistence, or message formats.

## Decisions

### Use the existing version metadata as the initial compatibility contract

The installed documents already carry a minimum CLI version and the version of
the CLI that rendered them. The checker will evaluate the skill and command
documents as a pair and classify the integration as `absent`, `current`,
`stale` (installed CLI is newer), `incompatible` (installed CLI is older than
the declared minimum or the documents are newer than the binary), or `unknown`
(partial files, missing fields, invalid fields, or inconsistent metadata). A
newer binary is not assumed to be compatible because the public command surface
may have been intentionally narrowed.

An explicit command-surface fingerprint is deferred. It would improve the
specificity of diagnostics, but it would also require deciding how manually
edited documents and semantically changed flags are represented. Version
metadata plus `tala --help` is sufficient for this warning-oriented first
iteration.

### Locate one shared project root

All integration operations will resolve one project root by walking from the
current working directory toward its ancestors and selecting the nearest
directory containing `.tala/config.json` or `.opencode/`. If no marker exists,
the current directory is the root for a normal `tala init`, and checking remains
quiet because no integration exists. The resolver will not scan siblings,
parent projects beyond the selected root, or the user's global configuration.
Using one resolver prevents a warning from one directory while refresh writes to
another.

### Run a non-blocking preflight on normal invocations

The command runner will perform the integration check before dispatching normal
commands, alongside the existing daemon compatibility precheck. Diagnostics
will be one-line warnings on stderr and will never prevent an otherwise valid
command from running. The checker will aggregate multiple pair problems into a
single warning. `--json` stdout remains reserved for the command's data; the
warning is not embedded in the JSON document.

The corrective commands are exempt from the normal stale warning or handle
their own status: `tala init`, `tala init --check`, `tala init --refresh`,
`tala --help`, and `tala --version`.

### Handle unknown subcommands separately

Clap rejects an unknown subcommand before the normal command runner executes.
The top-level entry point will therefore use a fallible parse path, preserve
Clap's normal usage text and exit status, and add the same stale-integration
hint when the current project has unversioned, incompatible, or stale Tala
documents. The hint will direct the user to `tala --help` and the explicit
refresh flow; it will not suggest restoring the rejected command. Help and
version requests bypass project diagnostics.

### Separate checking from writing during initialization

`tala init` will create missing integration files but preserve existing ones.
The mutually exclusive `--check` and `--refresh` flags will not accept a
positional identity name. Check mode will emit a single status document in
human-readable or `--json` form, report the selected project root, and leave
all files unchanged. Refresh mode will render the current embedded templates
and replace the integration pair only after both documents validate. It will
leave `.tala/config.json` unchanged.

Refresh will stage both destination files before replacing either destination.
If the second replacement fails, it will restore the first destination from its
original contents so the pair remains at its prior version. Explicit refresh is
the intentional overwrite operation for locally customized integration files.

This makes the normal remediation path safe for source-controlled repositories:

```text
tala command
  └─ warning: integration is stale
       └─ tala init --check       inspect without changes
       └─ tala init --refresh    intentionally update generated files
```

### Keep validation at build time and diagnostics at runtime

The existing documentation tests that extract `tala` command references from
the canonical templates will remain the source-level guard against documenting
removed commands, and the test's command set will represent the reduced public
surface. Runtime checks will validate metadata and file presence, not
reimplement a Markdown parser. Initialization will use the existing rendered
template validation before replacing files.

The canonical public surface for this change is `init`, `use`, `send`, `wait`,
`history`, `listen`, `check`, `list`, `discover`, `close`, `pending`, `status`,
`stop`, and `session`. `stream` and `agents` are intentionally absent. The
README, embedded integration templates, and documentation tests must use this
same list; the daemon's private `daemon` entry point is not part of the agent
surface.

## Risks / Trade-offs

- [Repeated warnings may be noisy for agents that invoke Tala many times] → Keep
  the diagnostic to one concise stderr line and do not add persistent
  repository state merely to suppress it.
- [The agent may be using a global or cached skill that Tala cannot inspect] →
  Phrase the warning as a project-integration diagnostic, retain compatibility
  guidance in the generated skill, and make unknown-command errors actionable.
- [A user may have intentionally customized generated documents] → Preserve
  existing files during normal init and require an explicit refresh before
  replacement; refresh is an explicit overwrite operation.
- [A pair replacement may fail after one destination changes] → Stage both
  documents and restore the first destination if the second replacement fails.
- [Older documents lack the new metadata] → Treat them as unknown rather than
  compatible, warn without blocking, and provide the refresh path.
- [The source branch and installed binary may have different command surfaces]
  → Compare against the installed binary at runtime; never use repository
  source or project-controlled metadata as proof of binary identity.

## Migration Plan

1. Release the CLI with the checker and explicit initialization modes, while
   retaining compatibility with existing unversioned integrations.
2. On the next Tala invocation in an existing repository, print an actionable
   warning when its integration is stale or unversioned.
3. Run `tala init --check` to inspect the repository, then
   `tala init --refresh` when the generated documents should be updated. Run
   these commands from any nested directory; both use the same resolved root.
4. Commit refreshed integration files with the repository when they are
   source-controlled.

Rolling back the CLI is safe: integration files remain Markdown and older
binaries will either use their supported fields or emit the existing
unversioned guidance. No daemon migration is required.
