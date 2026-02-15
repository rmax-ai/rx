# IMPLEMENT LIST FILES TOOL ALTERNATIVES PLAN

This plan defines safer, more explicit alternatives to `list_dir` for `rx`.

It is a planning/spec document only. No tool behavior changes are implied by this file.

---

## 1. Why this plan exists

Current `list_dir` is too coarse for many editing workflows.

Common failure modes:
- Agent must repeatedly traverse directories to find one target file.
- Large directories return noisy results with weak selection signals.
- Recursive intent is forced into iterative ad hoc loops.

Goal:
- Introduce focused discovery tools that reduce scan noise, improve determinism, and keep tool calls auditable.

---

## 2. Guardrails and constraints

This plan must respect:
- `ARCHITECTURE.md` (filesystem side effects stay in tool layer)
- `KERNEL.md` (kernel remains small, deterministic)
- `ROADMAP.md` phase order (durability before distribution)
- `AGENTS.md` determinism + replay requirements

Design constraints:
- No hidden traversal behavior
- Structured filters only (no free-form shell semantics)
- Stable, deterministic ordering in outputs
- Bounded result sets with explicit truncation metadata

---

## 3. Proposed alternative tools

### A) `list_dir_entries`
Use case:
- Explicit, non-recursive directory listing with richer metadata.

Behavior:
- Lists direct children only.
- Supports `include_hidden` (default `false`).
- Returns normalized entry fields: `name`, `path`, `kind`, `size`, `modified_unix_ms`.
- Sorts deterministically (`kind`, then `name`).

Why:
- Keeps current semantics but improves file selection quality and replay clarity.

---

### B) `find_files`
Use case:
- Recursive file discovery from a root path.

Behavior:
- Traverses recursively with explicit `max_depth`.
- Supports optional filters: `extensions`, `name_contains`, `path_contains`, `exclude_dirs`.
- Returns files only.
- Deterministic path ordering.
- Returns `truncated: true` with `next_cursor` if `limit` reached.

Why:
- Replaces repeated manual traversal loops with one bounded, explicit operation.

---

### C) `glob_search`
Use case:
- Pattern-based discovery when exact naming structure is known.

Behavior:
- Accepts constrained glob pattern syntax.
- Optional `root`, `kind` (`file`/`dir`/`any`), `max_results`.
- Deterministic ordering and explicit truncation metadata.
- Rejects invalid/ambiguous patterns.

Why:
- Makes targeted discovery concise without invoking shell tools.

---

### D) `stat_path`
Use case:
- Validate candidate path before read/write operation.

Behavior:
- Returns existence + metadata for a single path.
- Distinguishes `file`, `dir`, `symlink`, and `missing`.

Why:
- Prevents tool-call chains from failing late due to path-type ambiguity.

---

### E) `list_dir` (retained, narrowed)
Use case:
- Quick local inspection of one directory.

Behavior target:
- Keep basic direct listing behavior.
- Mark as non-preferred when recursive or filtered discovery is needed.

Why:
- Preserve backward compatibility while shifting default behavior to safer and more explicit alternatives.

---

## 4. Selection policy (agent guidance)

Preferred order:
1. `stat_path` to confirm path assumptions
2. `find_files` for recursive discovery
3. `glob_search` when known path pattern exists
4. `list_dir_entries` for local directory inspection
5. `list_dir` as legacy fallback

Prompt/tooling should discourage iterative blind traversal when a bounded discovery tool fits.

---

## 5. Contract-level requirements

Each alternative tool must:
- Return structured result including `root`, `query`, `count`, and `truncated`
- Guarantee deterministic ordering for equal filesystem state
- Expose explicit failure reasons (invalid root, permission denied, bad pattern)
- Avoid hidden recursion or implicit excludes
- Emit enough metadata for replay audits

Recommended result shape:
```json
{
  "success": true,
  "operation": "find_files",
  "root": "src",
  "count": 3,
  "truncated": false,
  "entries": [
    {
      "path": "src/main.rs",
      "name": "main.rs",
      "kind": "file",
      "size": 4210
    }
  ]
}
```

---

## 6. Phase-aligned rollout plan

### Phase 1 (allowed now)
1. Add `list_dir_entries`
2. Add `find_files`
3. Add `glob_search`
4. Add `stat_path`
5. Keep `list_dir` as compatibility tool
6. Update prompt + contract docs to prefer bounded discovery tools

Exit checks:
- Typical file-targeting workflows complete without repeated directory crawl loops.

### Phase 2 (durability)
1. Record normalized discovery query metadata in durable event log
2. Add replay checks for deterministic ordering + truncation behavior
3. Add resume-safe pagination/cursor semantics

Exit checks:
- Replayed runs produce identical discovery result ordering for same filesystem snapshot.

### Phase 3 (distributed runtime)
1. Serialize discovery queries as transport-safe payloads
2. Ensure worker-side filesystem normalization is consistent across environments

Exit checks:
- Remote discovery preserves local semantics and deterministic ordering guarantees.

---

## 7. Testing plan

Behavioral tests should focus on:
- Success path for each discovery tool
- Deterministic ordering across repeated calls
- Correct truncation + cursor behavior at limits
- Refusal on invalid patterns/filters
- Replay equivalence (same filesystem snapshot + same query => same ordered result)

Failure injection cases:
- Missing root directory
- Permission denied directory
- Symlink loops / recursive cycle protection
- Overly broad glob query exceeding limits

---

## 8. Documentation updates required at implementation time

When implemented, update:
- `CONTRACT.md` (discovery tool semantics + determinism guarantees)
- `LOOP_PROMPT.md` (tool selection hierarchy for path discovery)
- `CLI_SPEC.md` (if user-facing debug/inspection commands change)
- `TEST_GUIDELINES.md` (new deterministic discovery scenarios)

---

## 9. Non-goals

- No semantic code indexer in this phase
- No language-server dependency for file discovery
- No shell passthrough as default discovery path

Keep discovery primitives minimal, bounded, deterministic, and auditable.
