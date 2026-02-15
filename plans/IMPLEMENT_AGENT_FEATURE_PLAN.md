# IMPLEMENT AGENT FEATURE PLAN

This document proposes a minimal, Phase-1-safe agent selection feature for `rx`.

Scope of this plan:
- Add CLI support for `--agent <name>`
- Add config support for an `agent` section with:
  - `name`
  - `model`
  - `cli_defaults_overrides`

This is a planning/spec document only.

---

## 1. Why this exists

Current config supports global defaults, but not a named local agent profile.

The `--agent` feature enables:
- explicit run intent (`rx --agent writer ...`)
- profile-specific model selection
- profile-specific default CLI behavior without changing global defaults

Design goal: keep this as transport/config wiring only, with no kernel expansion.

---

## 2. Guardrails

This plan must preserve:
- `ARCHITECTURE.md`: no side effects or environment logic moved into kernel
- `KERNEL.md`: deterministic loop behavior unchanged
- `ROADMAP.md` Phase 1 constraints: local only, no resume enablement beyond current rules
- `AGENTS.md`: determinism, replayability, observability

Non-goals:
- No multi-agent orchestration
- No planner/executor role split
- No distributed agent runtime
- No prompt-only behavior changes as the primary mechanism

---

## 3. Proposed CLI surface

### New flag

`--agent <name>`

Semantics:
1. If `--agent` is omitted, behavior remains unchanged.
2. If `--agent <name>` is provided, `rx` attempts to load and activate a matching local agent profile from config.
3. If no matching profile exists, fail fast with a clear error.

Minimal usage example:

```bash
rx --agent writer "Refactor config loading"
```

---

## 4. Proposed config schema

Add an optional top-level `agent` section:

```toml
[agent]
name = "writer"
model = "gpt-5.3-codex"

[agent.cli_defaults_overrides]
max_iterations = 80
tool_verbose = true
debug_log = "logs/rx-debug-{goal_id}.jsonl"
```

Notes:
- `agent.name` is the profile identifier matched by `--agent`.
- `agent.model` overrides main model only when that profile is active.
- `agent.cli_defaults_overrides` supports the same allowed keys as `[cli_defaults]`.
- Unknown keys inside `agent.cli_defaults_overrides` are ignored with warning.

---

## 5. Resolution and precedence rules

When `--agent` is **not** set:
1. Built-in defaults
2. `[cli_defaults]`
3. Explicit CLI flags

When `--agent` **is** set and profile matches:
1. Built-in defaults
2. `[cli_defaults]`
3. `[agent.cli_defaults_overrides]`
4. Explicit CLI flags

Model resolution with active profile:
1. `--model` CLI value (highest)
2. `agent.model`
3. existing config model fallback (`model_name`)
4. `OPENAI_MODEL`
5. hardcoded fallback

This preserves current “CLI wins” behavior.

---

## 6. Validation and failure behavior

Validation rules:
- `--agent` requires a value.
- `agent.name` must be non-empty if `agent` section is present.
- `agent.cli_defaults_overrides` values must match existing field types.
- Phase-1 guardrail for `resume` remains enforced (same as current behavior).

Failure behavior:
- Missing profile for requested `--agent` -> hard error and non-zero exit.
- Invalid agent config -> warning + ignore agent overlay, unless `--agent` explicitly requested (then hard error).

Rationale:
- Explicit request should fail loudly.
- Passive config issues should not break normal runs.

---

## 7. Implementation plan

1. **Config structs**
   - Extend `config_loader` with optional `AgentConfig`:
     - `name: Option<String>`
     - `model: Option<String>`
     - `cli_defaults_overrides: Option<CliDefaults>`
   - Keep current `CliDefaults` as the shared schema for override keys.

2. **CLI parsing**
   - Add parsing for `--agent <name>` in `main.rs`.
   - Include `--agent` in usage/help text.

3. **Agent activation logic**
   - Resolve requested agent name from CLI.
   - If provided, validate config has matching `agent.name`.
   - Compute effective defaults by overlaying `agent.cli_defaults_overrides` over existing defaults.

4. **Model selection integration**
   - Apply `agent.model` only when profile is active and `--model` is not set.
   - Preserve current small-model behavior and compatibility aliases.

5. **Deterministic observability**
   - Emit structured logs for:
     - agent requested
     - agent matched/mismatched
     - which defaults were overridden by agent overlay
   - Keep event ordering deterministic.

6. **Documentation updates**
   - Update `CLI_SPEC.md` with `--agent` option and precedence notes.
   - Update `CONFIG_SPEC.md` with `agent` schema and matching behavior.
   - Add short note in `README.md` for basic usage.

---

## 8. Testing plan

Add tests for:
- Config parse success for valid `agent` section
- Invalid `agent` config types
- `--agent` missing value
- `--agent` mismatch against config name
- Precedence with and without active agent overlay
- Model resolution precedence (`--model` > `agent.model` > existing chain)
- Phase-1 `resume` guardrail unaffected by agent overlay

Prefer table-driven tests to keep deterministic coverage compact.

---

## 9. Rollout strategy

Phase 1-compatible rollout:
1. Implement parser + matching + overlay behavior
2. Land tests
3. Update docs
4. Validate with `cargo test`

No migration required.

---

## 10. Out of scope (explicit)

- Multiple named profiles (`[agents.<name>]`) in this change
- Runtime switching between multiple active agents in one run
- Agent-specific tool allow/deny policies
- Any kernel-level behavior branching by agent role

Keep this feature as a minimal CLI/config overlay mechanism.
