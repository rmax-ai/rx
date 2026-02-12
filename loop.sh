#!/usr/bin/env bash
set -euo pipefail

# rx autonomous development loop
# Reads LOOP_PROMPT.md and iteratively drives development via opencode.

# -----------------------------
# Configuration
# -----------------------------

PROMPT_FILE="LOOP_PROMPT.md"
MAX_ITERATIONS=50
LOG_DIR="logs"
AGENT_NAME="rx"
MODEL="gpt-5"

# -----------------------------
# Validate Environment
# -----------------------------

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <goal>"
  exit 1
fi

if [[ ! -f "$PROMPT_FILE" ]]; then
  echo "Error: $PROMPT_FILE not found"
  exit 1
fi

if ! command -v opencode >/dev/null 2>&1; then
  echo "Error: opencode CLI not found in PATH"
  exit 1
fi

GOAL="$*"
TIMESTAMP=$(date +"%Y%m%d-%H%M%S")
RUN_ID="run-$TIMESTAMP"
RUN_DIR="$LOG_DIR/$RUN_ID"

mkdir -p "$RUN_DIR"

echo "==============================================="
echo "rx Autonomous Development Run"
echo "Goal: $GOAL"
echo "Run ID: $RUN_ID"
echo "==============================================="

# -----------------------------
# Loop Execution
# -----------------------------

for ((i=1; i<=MAX_ITERATIONS; i++)); do
  echo
  echo "-----------------------------------------------"
  echo "Iteration $i"
  echo "-----------------------------------------------"

  ITERATION_LOG="$RUN_DIR/iteration-$i.log"

  opencode run "$AGENT_NAME" \
    --model "$MODEL" \
    --preamble "$(cat "$PROMPT_FILE")" \
    "$GOAL" \
    | tee "$ITERATION_LOG"

  # Auto-commit after each iteration
  if ! git diff --quiet; then
    git add .
    git commit -m "rx iteration $i: $GOAL"
  fi

  # Stop early if agent invoked done tool
  if grep -q '"done"' "$ITERATION_LOG"; then
    echo "Termination detected."
    break
  fi
done

echo
echo "==============================================="
echo "Run Complete"
echo "Logs stored in $RUN_DIR"
echo "==============================================="

