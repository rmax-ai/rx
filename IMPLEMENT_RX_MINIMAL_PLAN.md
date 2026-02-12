# IMPLEMENT_RX_MINIMAL_PLAN

Status: Phase 1 — Minimal Core
Owner: rx
Objective: Replace opencode loop with native rx microkernel agent.

This plan is executable by an autonomous coding agent.

Rules:
- Work sequentially.
- Do not skip tasks.
- Commit after each task.
- Do not introduce features outside Phase 1 scope.
- Preserve architecture invariants.

---

# PHASE 1 — MINIMAL CORE (LOCAL, EPHEMERAL)

Goal:
A single-binary Rust agent that:
- Loads LOOP_PROMPT.md
- Executes observe → decide → act → verify loop
- Uses structured tools
- Logs JSONL events
- Stops deterministically

---

## TASK 1 — Bootstrap Crate

[x] Create Rust binary crate `rx`
[x] Add dependencies:
    - tokio
    - serde
    - serde_json
    - async-trait
    - chrono
    - anyhow
    - thiserror
[x] Create folder structure:

src/
  main.rs
  kernel.rs
  model.rs
  tool.rs
  state.rs
  event.rs
  tools/
    exec.rs
    fs.rs
    done.rs

Commit message:
"Initialize rx minimal crate structure"

---

## TASK 2 — Define Core Types

[x] Define Event struct in event.rs
[x] Define Tool trait in tool.rs
[x] Define ToolRegistry
[x] Define Action enum (ToolCall | Message)
[x] Define Model trait in model.rs
[x] Define StateStore trait in state.rs

Constraints:
- No environment access in kernel
- No persistence implementation beyond in-memory

Commit:
"Define core traits and types"

---

## TASK 3 — Implement In-Memory State

[x] Implement InMemoryStateStore
[x] Append-only Vec<Event>
[x] JSONL writer to logs/<goal-id>.jsonl
[x] Ensure every append writes to file

Constraints:
- No SQLite yet
- No resume support

Commit:
"Implement in-memory state with JSONL logging"

---

## TASK 4 — Implement Tools

### 4A — exec tool
[x] Implement tokio::process::Command
[x] Capture stdout, stderr, exit status
[x] Return structured JSON
[x] Handle timeout safely

### 4B — fs tools
[x] read_file
[x] write_file
[x] list_dir

### 4C — done tool
[x] Accept reason
[x] Return structured termination signal

Constraints:
- No implicit chaining
- No direct kernel mutation

Commit:
"Implement minimal local tools"

---

## TASK 5 — Implement Model Adapter

[x] Load LOOP_PROMPT.md
[x] Inject as system/preamble
[x] Send goal as user message
[x] Enable structured tool calling
[x] Parse structured tool responses
[x] Reject malformed output
[x] Return Action enum

Constraints:
- No raw shell execution
- No free-form parsing

Commit:
"Implement model adapter with structured tool calls"

---

## TASK 6 — Implement Kernel Loop

[x] Implement iteration counter
[x] Hard max iterations (default 50)
[x] Load state
[x] Call model.next_action
[x] Validate Action
[x] Invoke tool via registry
[x] Append Event
[x] Stop on:
      - done tool
      - iteration cap
      - fatal error
[x] Emit clear termination reason

Constraints:
- Kernel must not access environment
- Kernel must remain small

Commit:
"Implement minimal autonomous kernel loop"

---

## TASK 7 — CLI Entrypoint

[x] Parse goal from CLI argument
[x] Generate goal ID
[x] Load LOOP_PROMPT.md
[x] Initialize model
[x] Register tools
[x] Initialize state store
[x] Run kernel
[x] Exit with status code

Optional:
[x] Add --max-iterations flag
[x] Add --auto-commit flag

Commit:
"Implement CLI entrypoint"

---

## TASK 8 — Replace loop.sh

[x] Simplify loop.sh to:

#!/usr/bin/env bash
cargo build --release
./target/release/rx "$*"

[x] Remove opencode dependency
[x] Ensure behavior parity

Commit:
"Replace opencode loop with native rx binary"

---

## TASK 9 — Validation Pass

Agent must verify:

[x] Can create file via tool
[x] Can modify file
[x] Can run cargo check
[x] Logs JSONL events correctly
[x] Stops at iteration cap
[x] Stops on done
[x] No kernel environment leakage
[x] No unstructured execution

If any fail:
Return to corresponding task.

Commit:
"Phase 1 validation complete"

---

# DEFINITION OF DONE (PHASE 1)

The system is complete when:

- Single binary runs autonomous loop.
- Tool calls are structured.
- JSONL event log is append-only.
- Kernel < ~500 LOC.
- No distributed logic exists.
- No SQLite exists.
- Replay feasibility preserved (log sufficient).

---

# OUT OF SCOPE (STRICT)

Do NOT implement:

- SQLite persistence
- Resume execution
- Distributed tool workers
- Multi-agent planning
- Snapshotting
- Plugin loading systems
- Agent mesh
- Web UI

If a task attempts expansion:
Abort and refocus on Phase 1.

---

# SUCCESS CRITERIA

rx must:

- Modify files autonomously.
- Execute commands autonomously.
- Log every action.
- Stop deterministically.
- Be explainable in one page.
- Run fully offline (except model API).

---

# TRACKING RULE

After each task:
- Mark checkbox complete.
- Commit with single-purpose message.
- Do not batch tasks.

If progress stalls:
Re-evaluate architecture boundaries.

---

END OF PLAN

