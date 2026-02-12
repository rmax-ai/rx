# CONTRACT

This document defines the behavioral contract between:

- The Kernel
- The LLM
- The Tool Runtime
- The State Backend

It governs how `rx` behaves.

The architecture enforces structure.
The contract enforces behavior.

---

## 1. Agent Contract (LLM Behavior)

The agent must:

1. Prefer action over explanation.
2. Use tools whenever side effects are required.
3. Work iteratively (observe → decide → act → verify).
4. Make small, reversible changes.
5. Persist progress after every step.
6. Stop only when goal is achieved or blocked.
7. Never assume implicit execution.
8. Produce structured tool calls only.

The agent must NOT:

- Execute imaginary actions.
- Assume tool success without inspection.
- Loop indefinitely without progress.
- Emit free-form shell commands outside tool calls.

---

## 2. Tool Contract

Each tool must:

- Be stateless from kernel perspective.
- Accept structured input.
- Return structured output.
- Surface errors explicitly.
- Avoid hidden side effects.
- Be idempotent when possible.

A tool must not:

- Mutate kernel memory directly.
- Write to storage outside its declared behavior.
- Call other tools implicitly.

All tool invocations must be logged.

---

## 3. Kernel Contract

The kernel must:

- Validate model output before execution.
- Reject malformed tool calls.
- Enforce iteration cap.
- Persist every iteration.
- Handle tool errors deterministically.
- Emit termination reason.

The kernel must not:

- Execute raw model text.
- Bypass tool registry.
- Skip persistence.
- Depend on prompt for safety.

Architecture must enforce invariants.

---

## 4. State Contract

State must be:

- Append-only (event-driven).
- Reconstructable.
- Replayable.
- Observable.

Events must minimally include:

- Goal ID
- Iteration number
- Model decision
- Tool invocation
- Tool output
- Error (if any)
- Termination status

No silent mutations.

---

## 5. Termination Contract

Execution must stop when:

- `done(reason)` tool is invoked.
- Max iterations exceeded.
- Fatal error encountered.
- No-progress detected.
- Explicit cancellation requested.

Termination must be explicit and persisted.

---

## 6. Determinism Rule

Given:

- Same goal
- Same tool outputs
- Same event history

The kernel must produce the same next step.

Non-determinism must be isolated to model inference only.

---

## 7. Growth Rule

If behavior cannot be explained by:

- Kernel
- Tool
- State
- Transport

Then responsibility boundaries are leaking.

The solution is refactoring, not adding layers.

---

## 8. Enforcement Principle

Safety and correctness must be enforced in code.

Never rely solely on:
- Prompt wording
- Model alignment
- Human supervision

`rx` is a systems agent.
Behavior must be structurally constrained.

---

## Final Constraint

If `rx` cannot:

- Be reasoned about step-by-step,
- Be audited via event log,
- Be restarted safely,
- Be described without referencing the model prompt,

It violates the contract.

