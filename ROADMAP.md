# ROADMAP

This document defines the controlled evolution of `rx`.

No speculative features.
No parallel expansion.
Each phase must stabilize before advancing.

---

## Phase 1 — Minimal Core (Local, Ephemeral)

Goal:
A working autonomous loop with local tools and in-memory state.

Deliverables:
- Kernel loop
- Tool registry
- Basic tool set:
  - exec
  - read_file
  - write_file
  - list_dir
  - done
- Hard iteration cap
- Structured logging (stdout + JSONL)
- CLI transport

Constraints:
- Single binary
- No database
- No distributed execution
- No resume support

Exit Criteria:
- Agent can modify files.
- Agent can run commands.
- Agent logs every step.
- Agent terminates deterministically.

---

## Phase 2 — Durable Execution

Goal:
Make execution restartable and replayable.

Deliverables:
- SQLite-backed event log
- StateStore trait implementation
- Resume from last iteration
- Deterministic replay mode
- Explicit goal IDs

Constraints:
- Still single-node
- No distributed tools yet

Exit Criteria:
- Kill process mid-run.
- Restart.
- Execution resumes correctly.
- Full event history reconstructable.

---

## Phase 3 — Distributed Tool Runtime

Goal:
Decouple kernel from tool execution.

Deliverables:
- Tool call serialization
- Event-driven tool responses
- Worker process abstraction
- Pluggable transport layer
- Stateless kernel option

Constraints:
- No orchestration framework
- No message broker dependency required
- Must still run locally without network

Exit Criteria:
- Kernel can run without direct environment access.
- Tools can execute in separate process.
- Kernel resumes when tool response arrives.

---

## Phase 4 — Multi-Agent Extension (Optional)

Goal:
Separate reasoning responsibilities.

Possible directions:
- Planner / Executor split
- Verifier agent
- Policy enforcement layer
- Guard agent

Constraints:
- Must not enlarge kernel
- Must reuse existing event model

Exit Criteria:
- Multiple agents operate on same goal context.
- Arbitration remains deterministic.

---

## Phase 5 — Snapshot & Forking (Advanced)

Goal:
Enable branching execution.

Deliverables:
- Checkpoint snapshots
- Fork execution graph
- Diff comparison
- Replay from arbitrary step

This phase is optional and only justified by real use cases.

---

## Evolution Rules

- No phase overlap.
- Do not build distributed before durable.
- Do not build multi-agent before distributed.
- Do not optimize before correctness.
- Architecture changes must be reflected in ARCHITECTURE.md.

---

## Guardrail

If complexity increases but:
- Observability decreases
- Determinism decreases
- Replay becomes harder

The change must be rejected.

---

## Principle

Grow only when forced by real constraints.

Minimal core first.
Durability second.
Distribution third.
Complexity last.

