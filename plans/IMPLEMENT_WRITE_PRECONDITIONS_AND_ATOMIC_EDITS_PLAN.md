# IMPLEMENT WRITE PRECONDITIONS AND ATOMIC EDITS PLAN

This plan defines deterministic write-safety upgrades for `rx`:
- stale-read protection via explicit write preconditions
- atomic file write/edit behavior

It is a planning/spec document only. No tool behavior changes are implied by this file.

---

## 1. Why this plan exists

Current `write_file` safety checks reduce destructive overwrites, but they do not prevent concurrent stale writes.

Common failure modes:
- Agent A reads file, Agent B updates file, Agent A overwrites based on stale snapshot.
- Multi-step edits partially apply if process stops mid-write.
- Full-file rewrites replace content unexpectedly under concurrent activity.

Goal:
- Make write operations fail closed when read assumptions are stale.
- Ensure writes/edits are atomic at file boundary.
- Preserve deterministic, auditable outcomes for replay.

---

## 2. Guardrails and constraints

This plan must respect:
- `ARCHITECTURE.md` (file side effects remain in tools)
- `KERNEL.md` (no kernel enlargement for locking orchestration)
- `ROADMAP.md` phase order (durability before distributed behavior)
- `AGENTS.md` determinism and replay requirements

Design constraints:
- No long-lived global lock manager in kernel
- No hidden retries that mask conflicts
- Structured, explicit conflict errors only
- Same input + same file state => same tool result

---

## 3. Proposed contract changes

### A) Read metadata surface (for precondition tokens)

`read_file` should optionally return metadata used for later write preconditions:
- `content_hash` (stable hash of full content)
- `mtime_unix_ms`
- `size_bytes`

Why:
- Enables compare-and-write without introducing lock lifecycle complexity.

Notes:
- Hash algorithm must be fixed in contract (for example SHA-256) to preserve replay determinism.

---

### B) Write preconditions on `write_file`

Add optional precondition object:

```json
{
  "precondition": {
    "expected_hash": "...",
    "expected_mtime_unix_ms": 1730000000000,
    "expected_size_bytes": 1024,
    "require_all": false
  }
}
```

Semantics:
- If no precondition provided: current behavior.
- If provided: evaluate before write.
- `require_all=false` (default): all supplied fields must match; omitted fields ignored.
- `require_all=true`: require all supported fields to be present and match.
- On mismatch: return explicit conflict error and perform no write.

Conflict result shape (example):
```json
{
  "success": false,
  "error": "precondition_failed",
  "path": "src/main.rs",
  "expected": { "hash": "..." },
  "actual": { "hash": "...", "mtime_unix_ms": 1730000000123, "size_bytes": 1055 }
}
```

Why:
- Optimistic concurrency control is simpler and more robust than manual lock/unlock tools.

---

### C) Atomic write semantics

For modes that replace file content (`create`, `overwrite` and future edit tools):

1. Write to temp file in same directory.
2. `fsync` temp file.
3. Atomically rename temp -> target.
4. Best-effort directory sync where supported.

Requirements:
- No partial target content visible.
- On failure, target file remains previous valid version.
- Temp files are cleaned up on normal and error paths.

Notes:
- `append` is not full atomic replacement; keep existing semantics and document clearly.

---

### D) Atomic edit semantics for patch/replace tools

For `replace_in_file` / `apply_unified_patch` style tools (planned/implemented separately):
- Read current snapshot.
- Validate match/context.
- Produce full new content in memory.
- Commit via same atomic replacement path.

Why:
- Edit tools inherit identical crash-safety and determinism guarantees.

---

## 4. Selection policy (agent guidance)

Preferred sequence for concurrent-safe updates:
1. `read_file` (capture content + metadata)
2. Prepare edit
3. `write_file` with precondition
4. On `precondition_failed`, re-read and re-plan

Avoid introducing explicit lock/unlock flows unless proven necessary by measured contention.

---

## 5. Determinism and replay rules

Tool events must record enough fields to replay conflict outcomes:
- precondition values supplied by caller
- observed file metadata at evaluation time
- conflict vs success outcome
- atomic write strategy used

Replay policy:
- In pure replay mode, preserve recorded outcomes from log.
- In live mode, evaluate against current filesystem and return real conflict/success.

---

## 6. Phase-aligned rollout plan

### Phase 1 (allowed now)
1. Add read metadata output (`hash`, `mtime`, `size`).
2. Add optional `precondition` to `write_file`.
3. Implement atomic replacement path for create/overwrite.
4. Add explicit `precondition_failed` error contract.
5. Document append non-atomic caveat.

Exit checks:
- Stale write attempts fail deterministically.
- Overwrite/create operations are atomic at file boundary.

### Phase 2 (durability)
1. Persist full precondition input and evaluation metadata in event log.
2. Add replay tests for conflict and success scenarios.
3. Add restart safety checks for temp-file cleanup/recovery.

Exit checks:
- Replayed runs preserve recorded conflict/success decisions.

### Phase 3 (distributed runtime)
1. Serialize preconditions as transport-safe payloads.
2. Ensure worker-side atomic commit semantics match local semantics.
3. Standardize cross-platform conflict metadata normalization.

Exit checks:
- Remote writes preserve local precondition and atomicity guarantees.

---

## 7. Testing plan

Behavioral tests should focus on:
- Success path with matching preconditions.
- Refusal path with stale hash/mtime/size.
- No-write guarantee on precondition failure.
- Atomic overwrite integrity under induced failure between temp write and rename.
- Concurrent writer race simulation: one succeeds, stale writer gets conflict.

Failure injection cases:
- Target deleted between read and write.
- Target replaced by directory/symlink.
- Temp file creation failure (permission/disk full).
- Rename failure/cross-device edge handling.

---

## 8. Documentation updates required at implementation time

When implemented, update:
- `CONTRACT.md` (precondition and atomicity guarantees)
- `CLI_SPEC.md` (if user-facing flags/commands expose precondition options)
- `LOOP_PROMPT.md` (read→preconditioned-write workflow guidance)
- `TEST_GUIDELINES.md` (race/conflict and crash-safety scenarios)

---

## 9. Non-goals

- No kernel-managed lock table.
- No distributed lock service.
- No long-lived file lease protocol in Phase 1.
- No hidden auto-merge on conflict.

Keep concurrency safety optimistic, explicit, and minimal.
