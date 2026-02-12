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

[ ] Create Rust binary crate `rx`
[ ] Add dependencies:
    - tokio
    - serde
    - serde_json
    - async-trait
    - chrono
    - anyhow
    - thiserror
[ ] Create folder structure:

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

[ ] Define Event struct in event.rs
[ ] Define Tool trait in tool.rs
[ ] Define ToolRegistry
[ ] Define Action enum (ToolCall | Message)
[ ] Define Model trait in model.rs
[ ] Define StateStore trait in state.rs

Constraints:
- No environment access in kernel
- No persistence implementation beyond in-memory

Commit:
"Define core traits and types"

---

## TASK 3 — Implement In-Memory State

[ ] Implement InMemoryStateStore
[ ] Append-only Vec<Event>
[ ] JSONL writer to logs/<goal-id>.jsonl
[ ] Ensure every append writes to file

Constraints:
- No SQLite yet
- No resume support

Commit:
"Implement in-memory state with JSONL logging"

---

## TASK 4 — Implement Tools

### 4A — exec tool
[ ] Implement tokio::process::Command
[ ] Capture stdout, stderr, exit status
[ ] Return structured JSON
[ ] Handle timeout safely

### 4B — fs tools
[ ] read_file
[ ] write_file
[ ] list_dir

### 4C — done tool
[ ] Accept reason
[ ] Return structured termination signal

Constraints:
- No implicit chaining
- No direct kernel mutation

Commit:
"Implement minimal local tools"

---

## TASK 5 — Implement Model Adapter

[ ] Load LOOP_PROMPT.md
[ ] Inject as system/preamble
[ ] Send goal as user message
[ ] Enable structured tool calling
[ ] Parse structured tool responses
[ ] Reject malformed output
[ ] Return Action enum

Constraints:
- No raw shell execution
- No free-form parsing

Commit:
"Implement model adapter with structured tool calls"

---

## TASK 6 — Implement Kernel Loop

[ ] Implement iteration counter
[ ] Hard max iterations (default 50)
[ ] Load state
[ ] Call model.next_action
[ ] Validate Action
[ ] Invoke tool via registry
[ ] Append Event
[ ] Stop on:
      - done tool
      - iteration cap
      - fatal error
[ ] Emit clear termination reason

Constraints:
- Kernel must not access environment
- Kernel must remain small

Commit:
"Implement minimal autonomous kernel loop"

---

## TASK 7 — CLI Entrypoint

[ ] Parse goal from CLI argument
[ ] Generate goal ID
[ ] Load LOOP_PROMPT.md
[ ] Initialize model
[ ] Register tools
[ ] Initialize state store
[ ] Run kernel
[ ] Exit with status code

Optional:
[ ] Add --max-iterations flag
[ ] Add --auto-commit flag

Commit:
"Implement CLI entrypoint"

---

## TASK 8 — Replace loop.sh

[ ] Simplify loop.sh to:

#!/usr/bin/env bash
cargo build --release
./target/release/rx "$*"

[ ] Remove opencode dependency
[ ] Ensure behavior parity

Commit:
"Replace opencode loop with native rx binary"

---

## TASK 9 — Validation Pass

Agent must verify:

[ ] Can create file via tool
[ ] Can modify file
[ ] Can run cargo check
[ ] Logs JSONL events correctly
[ ] Stops at iteration cap
[ ] Stops on done
[ ] No kernel environment leakage
[ ] No unstructured execution

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

