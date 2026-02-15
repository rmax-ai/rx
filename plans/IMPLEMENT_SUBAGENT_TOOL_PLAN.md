# IMPLEMENT SUBAGENT TOOL PLAN

This document defines a phase-gated plan for adding a `subagent` tool to `rx`.

Scope of this document:
- Specify a `subagent` tool that can invoke a fresh agent instance with isolated context
- Define deterministic input/output contracts for replay and audit
- Outline rollout and testing by roadmap phase

This is a planning/spec document only.

---

## 1. Why this plan exists

Some tasks are easier to complete by delegation:
- focused research
- narrow codebase scans
- bounded synthesis over a temporary context window

A `subagent` tool enables delegation without expanding kernel responsibilities.

Design goal:
- Keep orchestration in tools and transport-safe contracts
- Keep kernel unchanged except normal tool dispatch and event logging

---

## 2. Guardrails and phase constraints

This plan must preserve:
- `ARCHITECTURE.md`: side effects remain in tool layer
- `KERNEL.md`: kernel stays deterministic and small
- `AGENTS.md`: replayability, observability, and no architectural drift

Roadmap constraints:
- Multi-agent behavior is Phase 4 in `ROADMAP.md`
- Therefore, this plan is split into:
  - **Phase 2/3 prep**: schema, logging, and no-op/disabled wiring
  - **Phase 4 activation**: actual subagent execution

Non-goals for this plan:
- No distributed mesh/orchestration framework
- No concurrent swarm scheduling
- No prompt-only control plane

---

## 3. Tool contract (proposed)

### Tool name
`subagent`

### Input schema (v1)
- `task` (string, required): Delegated objective for the subagent
- `context` (string, optional): Additional scoped context payload
- `files` (array<string>, optional): File hints for targeted reading
- `max_iterations` (integer, optional): Subagent iteration cap override
- `timeout_seconds` (integer, optional, default 120)

### Output schema (v1)
- `success` (boolean)
- `summary` (string): Compact human-readable result
- `artifacts` (array<object>): Structured outputs (optional)
  - `{ "kind": "note|path|diff|json", "value": "..." }`
- `error` (string, optional)
- `timed_out` (boolean, optional)
- `duration_ms` (integer)

### Determinism requirements
- Stable response shape for all outcomes
- Explicit timeout and failure fields
- Normalized metadata order for replay comparison

---

## 4. Execution model

### Isolation model
- Subagent receives a fresh context payload composed from:
  1. Delegated `task`
  2. Optional `context`
  3. Optional file hints
- Parent kernel state is not directly mutable by subagent
- Parent only observes returned output payload

### Invocation model
- `subagent` tool acts as adapter around a child-agent runner
- Child runner can be local process or in-process adapter (phase-dependent)
- Parent iteration blocks until tool result is returned or timed out

### Failure model
- Child start failure -> structured `error`, `success: false`
- Child timeout -> `timed_out: true`, `success: false`
- Child malformed output -> mapped to deterministic validation error

---

## 5. Observability and replay contract

The tool must emit structured metadata for each call:
- correlation id
- delegated task hash (or normalized preview)
- timing (`started_at`, `duration_ms`)
- result status (`success`, `timed_out`, `error`)

Replay requirements:
- Event log captures complete tool input and normalized output
- Replays must not depend on hidden runtime globals
- Failure semantics remain stable across runs for identical recorded events

---

## 6. Phase-aligned rollout

### Phase 2 (prep only)
1. Add spec docs and contract entries for `subagent`
2. Add disabled tool registration mode behind explicit feature flag/config
3. Add schema validator and deterministic error payloads

Exit criteria:
- Calls to disabled `subagent` return clear structured "not enabled in this phase" errors

### Phase 3 (transport/runtime prep)
1. Define serialization envelope for delegated request/response
2. Add transport-safe payload constraints (size/time limits)
3. Add normalization helpers for replay-safe logging

Exit criteria:
- Delegated payload schema is stable and documented
- Logging and replay scaffolding complete

### Phase 4 (activation)
1. Implement child-agent runner adapter
2. Enable `subagent` tool execution path
3. Add guardrails: max depth, no recursive runaway, bounded iterations/time
4. Add policy for allowed returned artifact kinds

Exit criteria:
- Parent agent can delegate and receive deterministic structured output
- Nested delegation guardrails prevent infinite recursion

---

## 7. Safety and guardrails

Required guardrails:
- `max_depth` (default 1) to prevent recursive delegation loops
- hard timeout cap
- hard delegated iteration cap
- output size cap with truncation metadata
- explicit validation of returned payload schema

Recommended defaults:
- depth: 1
- timeout: 120 seconds
- delegated max iterations: small fixed cap (e.g., 8)

---

## 8. Minimal implementation tasks (when phase allows)

1. Add `src/tools/subagent.rs` with schema + deterministic error handling
2. Register tool in `src/tools/mod.rs` and `src/main.rs`
3. Introduce child-runner trait (tool-layer adapter only)
4. Wire structured logging fields for delegation events
5. Add behavioral tests for success/failure/timeout/guardrail cases
6. Update docs:
   - `CONTRACT.md`
   - `LOOP_PROMPT.md`
   - `evals/tools/subagent.md`

---

## 9. Testing strategy

Behavioral tests:
- successful delegation returns stable summary/artifacts shape
- timeout returns deterministic timeout payload
- malformed child output is normalized into schema error
- depth limit blocks recursive subagent loops deterministically
- repeated same mocked child response yields identical normalized output

Failure injection:
- child runner unavailable
- child runner returns invalid JSON
- child runner exceeds output cap

---

## 10. Open design decisions

1. Child runner location:
   - in-process adapter vs subprocess invocation
2. Artifact schema richness:
   - strict typed variants now vs minimal string payload first
3. Context packaging:
   - raw text only vs optional structured fields

Default recommendation:
- Start with minimal string-oriented contract and strict bounds, then evolve only if needed.

---

## 11. Out of scope

- Parallel multi-subagent scheduling
- Cross-agent shared mutable memory
- Distributed arbitration protocols
- Automatic planner/executor/verifier role graph

Keep `subagent` as a bounded delegation primitive with deterministic I/O.
