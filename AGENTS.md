# chit Eval Framework

Evaluate chit by running sub-agents through realistic multi-agent scenarios. Sub-agents use chit to communicate cross-project and then provide structured product feedback.

## Quick Start

```bash
./eval/run.sh setup cross-project
```

Follow the printed instructions to launch sub-agents. When they finish:

```bash
./eval/run.sh collect cross-project
```

Run cleanup when done:

```bash
./eval/run.sh cleanup
```

## Eval Scenarios

| Scenario | Description |
|---|---|
| `cross-project` | Two agents collaborate across projects via chit |
| `observe` | Multiple agents work; a monitor watches via `chit observe` |

## Adding a Scenario

1. Create `eval/scenarios/<name>.md` with:
   - `## Scenario` — narrative description
   - `## Setup` — expected directory structure and seed files
   - `## Agent Tasks` — one section per agent, describing their project context and goal
   - `## Feedback` — questions each agent should answer
2. Add a `setup_<name>` and `collect_<name>` function in `eval/run.sh`.

## How It Works

1. `./eval/run.sh setup <scenario>` — creates temp project dirs, starts chit daemon, writes sub-agent task files to `eval/agent-tasks/<scenario>/`
2. Coding agent reads task files and launches sub-agents via the Task tool
3. Sub-agents carry out their tasks using chit for cross-project messaging, then write feedback to a results file
4. `./eval/run.sh collect <scenario>` — reads feedback, stops daemon, prints summary
5. `./eval/run.sh cleanup` — removes temp dirs
