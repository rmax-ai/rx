# KERNEL

The kernel is the deterministic control center of `rx`.

It owns the autonomous loop and nothing else.

If the kernel grows complex, boundaries are being violated.

---

## Responsibility

The kernel is responsible for:

- Accepting a goal
- Maintaining execution state
- Running the iteration loop
- Invoking tools via registry
- Persisting structured events
- Evaluating termination conditions

The kernel does NOT:

- Execute shell commands directly
- Read or write files directly
- Access the network
- Implement storage logic
- Contain transport logic

---

## Core Loop

Each iteration follows this sequence:

1. Load current state
2. Generate next action (LLM)
3. Validate action
4. Invoke tool
5. Capture tool result
6. Append event
7. Evaluate termination
8. Increment iteration counter

Repeat until termination.

---

## Iteration Model

Each iteration must produce:

- Step number
- Model decision
- Tool invocation (if any)
- Tool input
- Tool output
- Error (if any)
- State summary

Every iteration must be persisted.

---

## Termination Conditions

Execution must stop when:

- `done` tool is invoked
- Maximum iterations exceeded
- Fatal tool error
- No progress detected
- Explicit cancellation

Termination must be logged with reason.

---

## Hard Constraints

The kernel must:

- Enforce max iteration cap
- Prevent infinite loops
- Handle tool failures gracefully
- Never panic on recoverable errors
- Emit structured events

Iteration cap must be enforced in code, not prompt.

---

## State Model (Kernel View)

Kernel state must minimally track:

- Goal
- Iteration count
- Event history reference
- Last tool result
- Termination status

The kernel must not depend on storage implementation.

---

## Model Interaction

The model produces structured action output.

The kernel must:

- Parse structured output
- Reject malformed responses
- Retry if necessary
- Never execute free-form text blindly

All tool calls must be explicit.

---

## Failure Handling

When a tool fails:

1. Log failure
2. Provide failure context to model
3. Allow model to retry or choose alternative
4. Abort if repeated failure exceeds threshold

Kernel must avoid silent failure loops.

---

## Resume Semantics (Future Phase)

The kernel must support:

- Reconstructing state from event log
- Resuming from last completed iteration
- Replaying events deterministically

Kernel logic must be replay-safe.

---

## Observability

Every kernel action must be:

- Logged (human-readable)
- Persisted (machine-readable)
- Traceable by goal ID

The system must allow:

- Step inspection
- Tool trace inspection
- Termination audit

---

## Minimal Kernel Rule

If removing a component makes:

- The system unable to decide next action → it belongs in kernel.
- The system unable to perform side effects → it belongs in tools.
- The system unable to persist history → it belongs in state backend.

Everything else is not kernel.

---

## Target Size (Phase 1)

The kernel implementation should be small.

If Phase 1 kernel exceeds a few hundred lines,
responsibilities are likely leaking.

The goal is clarity over feature richness.

