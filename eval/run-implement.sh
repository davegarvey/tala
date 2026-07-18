#!/usr/bin/env bash
# Run implement phase with P2-aware prompt (uses existing critic output)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

state_read
loop_num="${LOOP:-0}"
critic_file="$AGENT_TASKS_DIR/$SCENARIO/critic-output-loop-${loop_num}.json"

p0=$(jq '(.p0 | length)' "$critic_file" 2>/dev/null || echo "?")
p1=$(jq '(.p1 | length)' "$critic_file" 2>/dev/null || echo "?")
p2=$(jq '(.p2 | length)' "$critic_file" 2>/dev/null || echo "?")
if [ "$p0" = "?" ] || [ "$p1" = "?" ] || [ "$p2" = "?" ]; then total="?"; else total=$((p0 + p1 + p2)); fi

summary_file="$BASE_DIR/tmp/implement-summary-${loop_num}.json"
implement_prompt="$BASE_DIR/tmp/implement-prompt-${loop_num}.md"

cat > "$implement_prompt" << PROMPT
# Eval Fix Loop $loop_num — Spec & Implement

You are implementing fixes for issues found during the tala eval loop.

## Context

The eval scenario "$SCENARIO" has identified $total item(s) across all priorities:
- P0 (must fix): $p0
- P1 (should fix): $p1
- P2 (nice to have): $p2

## Your Tasks

1. **Triage all items.** Read the critic output, then decide for each item: fix it, defer to a future loop, or exclude with rationale. You have full remit to exclude any item (including P0/P1) if you judge it's not actionable, already fixed, or out of scope — just record your rationale.

2. **Propose a change name.** Based on the items you plan to fix, choose a short descriptive kebab-case name (e.g. "fix-csv-parsing", "add-error-handling"). Do not include the loop number — that will be added automatically.

3. Create the openspec change with your proposed name:
   - \`openspec new change <name>\`

4. Create all openspec artifacts for this change:
   - Read the critic output at: $critic_file
   - Run: \`openspec instructions proposal --change <name>\` and write the proposal file
   - Continue creating each artifact (specs, design, tasks) using \`openspec instructions\`
   - If openspec tells you to STOP, IGNORE that — continue until all artifacts exist

5. **Red-team the spec yourself** — review for gaps and flaws before implementing. Note what you find — you'll report it in the summary.

6. Implement the tasks from the tasks.md file

7. When done, commit all changes:
   - \`git add -A\`
   - \`git commit -m "<name>: implement fixes"\`

8. Write a JSON summary of what you did to: $summary_file
   Include fields: change_name, commits (array), files_changed (array), issues_fixed (array), excluded_items (array of {description, rationale} for each item you chose not to fix), red_team_findings (array of strings describing gaps/flaws you caught during red-teaming)
   Example:
   \`\`\`json
   {"change_name":"<your-proposed-name>","commits":["abc123"],"files_changed":["src/main.py"],"issues_fixed":["fixed csv parsing bug"],"excluded_items":[{"description":"rename alias inconsistency","rationale":"cosmetic only, low impact"}],"red_team_findings":["missing error handling for empty input"]}
   \`\`\`

Report what you did, what was fixed, what was excluded and why, and any red-team gaps you caught.
PROMPT

# Find server URL
SERVER_URL=""
if [ -f "$BASE_DIR/tmp/opencode-server.log" ]; then
  SERVER_URL=$(sed -n 's/.*\(http:\/\/[0-9.]*:[0-9]*\).*/\1/p' "$BASE_DIR/tmp/opencode-server.log" 2>/dev/null || true)
fi
if [ -z "$SERVER_URL" ] || ! curl -sf "$SERVER_URL/health" >/dev/null 2>&1; then
  echo "No running server found at $SERVER_URL"
  echo "Ensure opencode serve is running. The eval loop may have been killed."
  echo "Prompt written to $implement_prompt — you can run it manually:"
  echo "  opencode run --auto --attach <url> \"\$(cat $implement_prompt)\" --dir $SCRIPT_DIR/.."
  exit 1
fi

echo "Server OK at $SERVER_URL"
PROMPT_TEXT=$(cat "$implement_prompt")
MODEL_ARG=""
VARIANT_ARG=""
[ -n "$MODEL" ] && MODEL_ARG="--model $MODEL"
[ -n "$VARIANT" ] && VARIANT_ARG="--variant $VARIANT"
echo "Launching implementation agent (timeout: 1800s)..."
opencode run --auto --attach "$SERVER_URL" $MODEL_ARG $VARIANT_ARG "$PROMPT_TEXT" --dir "$SCRIPT_DIR/.." 2>>/tmp/implement-stderr.log
