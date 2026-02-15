# IMPLEMENT EXEC TOOL ALTERNATIVES PLAN

This plan defines safer, more explicit alternatives to `exec` for `rx`.

It is a planning/spec document only. No tool behavior changes are implied by this file.

---

## 1. Why this plan exists

Current `exec` is intentionally minimal, but too broad for many agent workflows.

Common failure modes:
- Tool calls over-capture output when only success/failure is needed.
- Large stdout/stderr payloads create noisy context and weak iteration signals.
- Preflight checks (command presence, working directory assumptions) are mixed into full command execution.

Goal:
- Introduce bounded, intention-revealing command tools that improve safety, observability, determinism, and replay audits.

---

## 2. Guardrails and constraints

This plan must respect:
- `ARCHITECTURE.md` (process execution remains in tool layer)
- `KERNEL.md` (kernel remains small, deterministic)
- `ROADMAP.md` phase order (durability before distribution)
- `AGENTS.md` determinism + replay requirements

Design constraints:
- No shell passthrough semantics by default
- Structured argv inputs only
- Explicit limits for runtime and output size
- Deterministic result ordering/shape for same process outcome
- Explicit truncation metadata when limits are hit

---

## 3. Proposed alternative tools

### A) `exec_status`
Use case:
- Health checks and validation commands where output is secondary.

Behavior:
- Inputs: `command`, `args`, optional `cwd`, optional `timeout_seconds`.
- Returns `success`, `exit_code`, `timed_out`, and bounded stderr summary.
- Does not return full stdout.

Why:
- Prevents oversized outputs for simple pass/fail checks.

---

### B) `exec_capture`
Use case:
- Run commands where stdout/stderr content is needed for reasoning.

Behavior:
- Inputs: `command`, `args`, optional `cwd`, optional `timeout_seconds`, `max_stdout_bytes`, `max_stderr_bytes`.
- Returns bounded stdout/stderr, exit metadata, and per-stream truncation flags.
- No shell expansion; command is executed directly.

Why:
- Keeps existing capability while making output bounds explicit and auditable.

---

### C) `exec_with_input`
Use case:
- Commands that require deterministic stdin payload (formatters, patch tools, validators).

Behavior:
- Inputs: `command`, `args`, optional `cwd`, `stdin`, optional `timeout_seconds`, output byte limits.
- Returns same bounded capture contract as `exec_capture`.

Why:
- Avoids fragile multi-step file-based workarounds when stdin is the natural interface.

---

### D) `which_command`
Use case:
- Resolve and validate executable availability before execution.

Behavior:
- Inputs: `command`.
- Returns `found`, resolved path (if found), and normalization metadata.

Why:
- Converts late command-not-found failures into explicit preflight checks.

---

### E) `exec` (retained, narrowed)
Use case:
- Backward-compatible command execution.

Behavior target:
- Keep current direct-process semantics.
- Mark as compatibility path; prefer explicit alternatives above.

Why:
- Preserves existing workflows while shifting default behavior to bounded, intention-revealing tools.

---

## 4. Selection policy (agent guidance)

Preferred order:
1. `which_command` to validate command assumptions
2. `exec_status` for probe/verification commands
3. `exec_capture` when output content is required
4. `exec_with_input` when deterministic stdin is required
5. `exec` as legacy fallback

Prompt/tooling should discourage broad capture when status-only intent is sufficient.

---

## 5. Contract-level requirements

Each alternative exec tool must:
- Return structured result including `operation`, `command`, `args`, `cwd`, `duration_ms`
- Guarantee explicit timeout semantics (`timed_out: true` on timeout)
- Report deterministic truncation metadata for bounded streams
- Distinguish process launch failure vs process exit failure
- Emit enough metadata for replay and audit

Recommended result shape:
```json
{
  "success": true,
  "operation": "exec_capture",
  "command": "cargo",
  "args": ["check"],
  "cwd": "/workspace",
  "exit_code": 0,
  "timed_out": false,
  "duration_ms": 842,
  "stdout": "...",
  "stderr": "...",
  "stdout_truncated": false,
  "stderr_truncated": false
}
```

---

## 6. Phase-aligned rollout plan

### Phase 1 (allowed now)
1. Add `which_command`
2. Add `exec_status`
3. Add `exec_capture`
4. Add `exec_with_input`
5. Keep `exec` as compatibility tool with lower selection priority
6. Update prompt + contract docs to prefer bounded execution paths

Exit checks:
- Typical build/test/probe workflows complete without oversized output capture.

### Phase 2 (durability)
1. Persist normalized execution metadata in durable event log
2. Add replay checks for timeout/truncation and output-bound determinism
3. Add resume-safe handling for interrupted execution events

Exit checks:
- Replayed runs preserve execution metadata semantics for same event stream.

### Phase 3 (distributed runtime)
1. Serialize execution requests as transport-safe payloads
2. Normalize worker-side timeout and truncation behavior
3. Preserve deterministic result schema across local and remote runtimes

Exit checks:
- Remote execution preserves local contract semantics and auditability.

---

## 7. Testing plan

Behavioral tests should focus on:
- Success path for each execution tool
- Deterministic timeout behavior
- Correct truncation flags at output limits
- Explicit error taxonomy (spawn failure, timeout, non-zero exit)
- Replay equivalence for normalized result metadata

Failure injection cases:
- Command not found
- Non-zero exit command
- Timeout expiration
- Invalid working directory
- Large stdout/stderr exceeding configured limits

---

## 8. Documentation updates required at implementation time

When implemented, update:
- `CONTRACT.md` (execution error taxonomy + bounded output semantics)
- `LOOP_PROMPT.md` (tool selection hierarchy for command execution)
- `CLI_SPEC.md` (if any debug/inspection behavior is exposed)
- `TEST_GUIDELINES.md` (bounded-exec and timeout scenarios)

---

## 9. Non-goals

- No shell parser or shell script emulation in this phase
- No background process supervisor/orchestration framework
- No policy engine for command allow/deny lists in this phase

Keep command execution primitives minimal, bounded, deterministic, and auditable.
