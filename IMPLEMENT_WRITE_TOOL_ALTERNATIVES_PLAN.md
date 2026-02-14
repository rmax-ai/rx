# IMPLEMENT WRITE TOOL ALTERNATIVES PLAN

This plan defines safer, more explicit alternatives to `write_file` for `rx`.

It is a planning/spec document only. No tool behavior changes are implied by this file.

---

## 1. Why this plan exists

Current `write_file` is high-risk when used as a full-file overwrite.

Common failure mode:
- Agent writes partial content (placeholders, omitted sections, truncated file), unintentionally destroying valid code.

Goal:
- Introduce smaller, intention-revealing write tools that reduce destructive edits and improve replayability.

---

## 2. Guardrails and constraints

This plan must respect:
- `ARCHITECTURE.md` (tool side effects remain in tool layer)
- `KERNEL.md` (kernel remains small, deterministic)
- `ROADMAP.md` phase order (no distributed logic before durable execution is complete)
- `AGENTS.md` determinism + replay requirements

Design constraints:
- No speculative framework additions
- No hidden side effects
- Structured inputs/outputs only
- Deterministic behavior for same input + file state

---

## 3. Proposed alternative tools

### A) `create_file`
Use case:
- Create new files only.

Behavior:
- Fails if target file already exists.

Why:
- Prevents accidental overwrite when intent is creation.

---

### B) `append_file`
Use case:
- Add content to end of file (logs, config tails, generated sections).

Behavior:
- Creates file if missing (explicit in contract).
- Never truncates existing content.

Why:
- Removes overwrite risk for additive operations.

---

### C) `replace_in_file`
Use case:
- Replace a known snippet with another snippet.

Behavior:
- Requires exact `old_text` match.
- Optionally supports `expected_matches` (default 1).
- Fails if mismatch/multiple unexpected matches.

Why:
- Safer than full rewrite; deterministic and auditable.

---

### D) `apply_unified_patch`
Use case:
- Multi-hunk edits across one or more files.

Behavior:
- Applies unified diff with strict context matching.
- Fails on context mismatch; no partial silent success.

Why:
- Standard, explicit edit format with strong safety semantics.

---

### E) `write_file` (retained, narrowed)
Use case:
- Full-file replacement when intentional.

Behavior target:
- Keep mode-based contract (`create` / `overwrite` / `append`) or deprecate broad overwrite path in favor of tools above.
- Require explicit force path for destructive truncation risk.

Why:
- Preserve backward compatibility while shifting default behavior to safer alternatives.

---

## 4. Selection policy (agent guidance)

Preferred order:
1. `replace_in_file` for localized changes
2. `apply_unified_patch` for multi-hunk edits
3. `append_file` for additive updates
4. `create_file` for new files
5. `write_file(overwrite)` only when full-content intent is explicit

Prompt/tooling should treat full-file overwrite as last resort.

---

## 5. Contract-level requirements

Each alternative tool must:
- Return structured result including file path, operation, bytes changed
- Report deterministic failure reasons (match failed, conflict, invalid args)
- Avoid partial hidden writes
- Emit enough metadata for replay audits

Recommended result shape:
```json
{
  "success": true,
  "operation": "replace_in_file",
  "path": "src/main.rs",
  "changed": true,
  "bytes_before": 1200,
  "bytes_after": 1214
}
```

---

## 6. Phase-aligned rollout plan

### Phase 1 (allowed now)
1. Add `create_file`
2. Add `append_file`
3. Add `replace_in_file`
4. Keep `write_file` with strict overwrite safeguards
5. Update prompt + contract docs to prefer non-destructive tools

Exit checks:
- Typical code-edit workflow can complete without full-file overwrite.

### Phase 2 (durability)
1. Record richer edit metadata in durable event log
2. Add replay validation for text-edit determinism
3. Add resume-safe conflict handling policy

Exit checks:
- Replayed runs produce identical file transitions for same tool sequence.

### Phase 3 (distributed runtime)
1. Serialize patch/replace operations as transport-safe payloads
2. Ensure worker-side application returns deterministic conflict reports

Exit checks:
- Remote tool execution preserves exact local semantics.

---

## 7. Testing plan

Behavioral tests should focus on:
- Success path for each tool
- Refusal on ambiguous/destructive input
- Deterministic mismatch errors
- Replay equivalence (same initial files + same tool events => same final files)

Failure injection cases:
- Stale context for `replace_in_file`
- Patch context drift for `apply_unified_patch`
- Existing target on `create_file`
- Nonexistent path parent handling

---

## 8. Documentation updates required at implementation time

When implemented, update:
- `CONTRACT.md` (tool safety rules)
- `LOOP_PROMPT.md` (tool selection hierarchy)
- `KERNEL.md` (no kernel scope change)
- `TEST_GUIDELINES.md` (new behavior scenarios)

---

## 9. Non-goals

- No AST-aware editor in this phase
- No language-server-coupled edit engine
- No speculative merge/orchestration framework

Keep text-editing primitives minimal, deterministic, and auditable.
