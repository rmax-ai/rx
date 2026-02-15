# IMPLEMENT READ FILE TOOL ALTERNATIVES PLAN

This plan defines safer, more explicit alternatives to `read_file` for `rx`.

It is a planning/spec document only. No tool behavior changes are implied by this file.

---

## 1. Why this plan exists

Current `read_file` returns whole-file content only.

Common failure modes:
- Large files create high token cost and noisy context.
- Agents over-read when only a small section is needed.
- Repeated full reads reduce iteration efficiency and observability quality.

Goal:
- Introduce bounded, intention-revealing read tools that improve precision, determinism, and replay audits.

---

## 2. Guardrails and constraints

This plan must respect:
- `ARCHITECTURE.md` (filesystem I/O remains in tool layer)
- `KERNEL.md` (kernel remains small, deterministic)
- `ROADMAP.md` phase order (durability before distribution)
- `AGENTS.md` determinism + replay requirements

Design constraints:
- No implicit pagination or hidden reads
- Structured query inputs only (no shell passthrough semantics)
- Deterministic ordering and stable range semantics
- Explicit truncation/limit metadata in responses

---

## 3. Proposed alternative tools

### A) `read_file_range`
Use case:
- Read specific lines from a file.

Behavior:
- Inputs: `path`, `start_line`, `end_line` (1-based, inclusive).
- Validates bounds and returns normalized range metadata.
- Returns exact lines requested when in range.

Why:
- Replaces full-file reads for targeted edits and verification.

---

### B) `read_file_head`
Use case:
- Quick file inspection for headers/config/imports.

Behavior:
- Inputs: `path`, `max_lines`.
- Returns first `max_lines` only.
- Includes `total_lines` when computable and `truncated` flag.

Why:
- Keeps discovery lightweight and deterministic.

---

### C) `read_file_tail`
Use case:
- Inspect log tails or recent appended sections.

Behavior:
- Inputs: `path`, `max_lines`.
- Returns last `max_lines` only.
- Includes `total_lines` when computable and `truncated` flag.

Why:
- Avoids large reads when only recent lines matter.

---

### D) `search_in_file`
Use case:
- Find line locations of a literal or regex pattern before targeted reads.

Behavior:
- Inputs: `path`, `query`, `is_regex`, optional `max_matches`.
- Returns line-numbered matches with bounded context (`before_lines`, `after_lines`).
- Deterministic match ordering by line number.

Why:
- Converts blind scanning into precise, auditable selection.

---

### E) `stat_file`
Use case:
- Validate file existence/type/size before a read strategy is chosen.

Behavior:
- Inputs: `path`.
- Returns `exists`, `kind`, `size_bytes`, `modified_unix_ms`.

Why:
- Prevents expensive or invalid reads and improves error clarity.

---

### F) `read_file` (retained, narrowed)
Use case:
- Full-file read when explicitly required.

Behavior target:
- Keep full content behavior.
- Encourage bounded alternatives by default in prompt and contract guidance.

Why:
- Preserve backward compatibility while reducing default over-read behavior.

---

## 4. Selection policy (agent guidance)

Preferred order:
1. `stat_file` to validate assumptions
2. `search_in_file` to locate target regions
3. `read_file_range` for exact extraction
4. `read_file_head` / `read_file_tail` for quick bounded inspection
5. `read_file` only when full file is genuinely required

Prompt/tooling should discourage repeated full-file reads for narrow tasks.

---

## 5. Contract-level requirements

Each alternative read tool must:
- Return structured result including `path`, `operation`, `count`, and `truncated`
- Guarantee deterministic range and match ordering for same file state
- Expose explicit errors (missing file, invalid range, invalid regex)
- Avoid hidden fallback behavior (no implicit whole-file read)
- Emit enough metadata for replay audits

Recommended result shape:
```json
{
  "success": true,
  "operation": "read_file_range",
  "path": "src/main.rs",
  "start_line": 40,
  "end_line": 60,
  "line_count": 21,
  "truncated": false,
  "content": "..."
}
```

---

## 6. Phase-aligned rollout plan

### Phase 1 (allowed now)
1. Add `read_file_range`
2. Add `read_file_head`
3. Add `read_file_tail`
4. Add `search_in_file`
5. Add `stat_file`
6. Keep `read_file` as compatibility tool and lower preference in guidance

Exit checks:
- Typical edit workflows succeed using bounded reads without repeated full-file scans.

### Phase 2 (durability)
1. Persist normalized read query metadata in durable event log
2. Add replay checks for deterministic range/match output
3. Add resume-safe truncation and limit semantics

Exit checks:
- Replayed runs produce identical read slices and search ordering for same file snapshot.

### Phase 3 (distributed runtime)
1. Serialize read/search queries as transport-safe payloads
2. Ensure worker-side line-splitting and regex behavior is normalization-safe

Exit checks:
- Remote reads preserve local deterministic semantics.

---

## 7. Testing plan

Behavioral tests should focus on:
- Success path for each bounded read tool
- Deterministic line slicing and match ordering
- Correct truncation metadata at limits
- Refusal on invalid range and invalid pattern input
- Replay equivalence (same snapshot + same read query => same result)

Failure injection cases:
- Missing file path
- Directory path passed to file read tool
- Out-of-bounds line range
- Regex parse failures
- Very large file with strict limits

---

## 8. Documentation updates required at implementation time

When implemented, update:
- `CONTRACT.md` (bounded read guarantees + deterministic semantics)
- `LOOP_PROMPT.md` (read-tool selection hierarchy)
- `CLI_SPEC.md` (if debug/inspection command behaviors change)
- `TEST_GUIDELINES.md` (bounded-read and replay scenarios)

---

## 9. Non-goals

- No AST-aware semantic reader in this phase
- No language-server dependency for basic file reads
- No implicit auto-chunking hidden from tool output

Keep read primitives minimal, bounded, deterministic, and auditable.
