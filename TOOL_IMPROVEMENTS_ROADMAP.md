# TOOL IMPROVEMENTS ROADMAP

This roadmap defines a phase-aligned evolution of `rx` tooling for autonomous coding reliability.

It complements `ROADMAP.md` and does not change kernel architecture boundaries.

---

## Objectives

Primary goals:
- Improve determinism of tool execution
- Improve replayability and auditability
- Reduce unsafe or over-broad tool effects
- Improve autonomous recovery from failures

Non-goals:
- No UI layer
- No speculative distributed orchestration before roadmap phase allows it
- No kernel logic moved into prompts

---

## Design Principles

All tool improvements must preserve:
- Explicit inputs and outputs
- Deterministic result shape
- Bounded side effects
- Structured traceability for replay

All tools should expose:
- Typed input schema
- Typed output schema
- Error taxonomy (spawn/config/timeout/runtime)
- Duration and truncation metadata when relevant

---

## Workstreams

1. Deterministic Task Runner
2. Structured Repo Indexer
3. Typed Tool Contracts
4. Safe Edit Engine
5. Verification Pipeline Tool
6. Replay + Trace Debugger
7. Policy Guardrails Tool
8. Self-Heal Failure Loop

---

## Phase Plan

### Phase 1 — Local Tool Correctness (Current focus)

Scope:
- Deterministic Task Runner (local)
- Typed Tool Contracts (core tools)
- Safe Edit Engine (minimal semantic safety checks)
- Verification Pipeline Tool (changed-files targeted checks)

Deliverables:
- Idempotent step execution metadata (`step_id`, `inputs_hash`, `attempt`)
- JSON-schema-like contract definitions for each tool result
- Minimal diff guardrails for file writes/patches
- One-command validation flow for changed files (`fmt`/`lint`/`build`/targeted tests)

Exit criteria:
- Re-running the same tool step with same inputs yields same normalized output contract
- Tool failures are classifiable and machine-actionable
- Patch operations reject broad unintended edits
- Agent can validate changes without running full-suite by default

---

### Phase 2 — Durability and Replay Safety

Scope:
- Replay + Trace Debugger
- Durable Task Runner event model
- Verification result persistence

Deliverables:
- Persisted action graph linking decision -> tool call -> result
- Replay view that explains why each action was chosen
- Deterministic normalization of volatile fields in logs

Exit criteria:
- A run interrupted mid-iteration can resume without semantic drift
- Replay reproduces equivalent decisions from the same event history
- Trace output supports root-cause analysis without external context

---

### Phase 3 — Decoupled/Distributed Tool Runtime

Scope:
- Policy Guardrails Tool (pre-execution enforcement)
- Structured Repo Indexer transport-safe snapshots
- Cross-runtime contract parity

Deliverables:
- Preflight policy checks before remote/local execution (path/network/secrets rules)
- Serialized tool requests/responses with versioned schema
- Deterministic repo index snapshots consumable by workers

Exit criteria:
- Tool policy violations fail before execution with explicit reasons
- Local and remote tool results are schema-compatible
- Index snapshots are reproducible for identical repository state

---

### Phase 4 — Autonomous Reliability Layer

Scope:
- Self-Heal Failure Loop
- Advanced verification strategy selection

Deliverables:
- Failure classifier mapping known error classes to fix playbooks
- Retry policies constrained by idempotency and policy checks
- Automatic escalation path when confidence is low

Exit criteria:
- Common failures (missing symbol/import, compile errors, flaky command invocation) auto-resolve within bounded retries
- Retries never bypass policy guardrails
- All auto-repair actions remain auditable and replay-safe

---

## Dependencies and Ordering

Required order:
1. Typed Tool Contracts before broad Self-Heal automation
2. Deterministic Task Runner before durable replay enhancements
3. Policy Guardrails before distributed execution expansion

Do not advance a phase if:
- Observability drops
- Replay semantics degrade
- Error handling becomes less explicit

---

## Acceptance Metrics

Track per phase:
- Tool-call determinism rate
- Replay equivalence rate
- Mean time to classify failure
- Mean retries to successful recovery
- Percentage of runs blocked by guardrails before unsafe execution

Metrics must be derivable from structured logs/events, not manual inspection.

---

## Documentation Updates Required During Implementation

When each workstream lands, update:
- `CONTRACT.md` for schema and error taxonomy
- `ARCHITECTURE.md` when boundaries or interfaces change
- `KERNEL.md` for invariants and replay assumptions
- `TEST_GUIDELINES.md` for failure-injection and determinism tests
- workstream-specific implementation plan files

---

## Guardrail

If a proposed tool feature cannot be explained with:
- one paragraph of behavior,
- one deterministic request/response example,
- one replay trace,

it should not be added in current form.
