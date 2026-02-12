# AGENTS

This document governs autonomous contributions to `rx`.

Agents are allowed to modify code, documentation, and tooling.
Agents are not allowed to violate architectural invariants.

This file supersedes informal instructions.

---

# 1. Mission

Develop and evolve `rx` into a minimal autonomous microkernel agent
capable of running locally and in distributed mode.

Primary objective:
Preserve clarity, determinism, and architectural boundaries.

Secondary objective:
Continuously improve development process and documentation.

---

# 2. Authority Model

Agents MAY:

- Modify Rust source files.
- Improve documentation.
- Refactor code.
- Add tests.
- Improve logging.
- Improve determinism.
- Remove unnecessary complexity.
- Improve tooling and CI.

Agents MUST NOT:

- Introduce architectural drift.
- Add speculative features.
- Add external dependencies without justification.
- Bypass the kernel boundaries.
- Move logic into prompt instead of code.
- Add distributed logic before Phase 2 is complete.

If uncertain, default to minimal change.

---

# 3. Architectural Guardrails

Before modifying code, agents must verify:

- Does this change enlarge the kernel?
- Does this couple layers?
- Does this introduce hidden state?
- Does this reduce replayability?
- Does this reduce observability?

If YES to any:
The change must be rejected or redesigned.

---

# 4. Development Loop Protocol

When implementing a change:

1. Read ARCHITECTURE.md.
2. Read KERNEL.md.
3. Confirm phase in ROADMAP.md.
4. Implement minimal change.
5. Run tests or simulations.
6. Update documentation if behavior changed.
7. Ensure logging reflects new behavior.
8. Commit in small increments.

Agents must prefer incremental evolution over large rewrites.

---

# 5. Phase Enforcement

Agents must respect current roadmap phase.

Allowed work depends on phase:

Phase 1:
- Local kernel stability.
- Tool correctness.
- Logging quality.

Phase 2:
- Persistence correctness.
- Replay safety.

Phase 3:
- Decoupling and transport abstraction.

Agents must not skip phases.

---

# 6. Self-Improvement Mandate

Agents are expected to improve:

- Documentation clarity.
- Architectural consistency.
- Code readability.
- Observability.
- Determinism.
- Failure handling.

Agents must periodically:

- Reconcile docs with implementation.
- Remove outdated sections.
- Simplify overly complex components.
- Improve test coverage via behavioral simulation.

Documentation must reflect reality.

---

# 7. Determinism and Replay Rule

All changes must preserve:

- Step-by-step traceability.
- Event log completeness.
- Replay feasibility.

If a feature breaks replay semantics,
it must be rejected.

---

# 8. Tooling Improvement Scope

Agents may improve:

- CLI ergonomics.
- Structured logs.
- Debug commands.
- Simulation harness.
- Failure injection utilities.

Agents must not:

- Add UI layers.
- Add unnecessary frameworks.
- Add agent mesh complexity.

---

# 9. Commit Discipline

Commits must be:

- Small
- Descriptive
- Single-purpose
- Linked to roadmap phase

Large refactors must be split.

---

# 10. Prompt Discipline

The prompt is not architecture.

Agents must:

- Avoid embedding system rules in prompt.
- Enforce invariants in code.
- Keep prompt minimal.

If behavior depends on prompt wording,
it is a design flaw.

---

# 11. Anti-Patterns

Agents must avoid:

- Overengineering.
- Adding features “just in case.”
- Premature distribution.
- Embedding business logic in tools.
- Silent state mutations.
- Expanding kernel responsibilities.

---

# 12. Refactoring Rule

If code becomes:

- Hard to reason about,
- Larger than expected,
- Cross-layer coupled,

Agents must refactor before adding features.

Stability precedes expansion.

---

# 13. Testing Philosophy

Prefer:

- Behavioral simulation
- Deterministic replay
- Failure injection
- End-to-end iteration tests

Avoid:

- Excessive brittle unit tests
- Testing implementation details
- Mocking the kernel heavily

Test system behavior, not internals.

---

# 14. Documentation Evolution

Agents must:

- Keep README minimal.
- Keep ARCHITECTURE precise.
- Keep KERNEL deterministic.
- Keep ROADMAP realistic.
- Keep CONTRACT enforceable.

Documentation must shrink when possible.

Complexity must not accumulate in text.

---

# 15. Growth Constraint

`rx` must remain:

- Small
- Deterministic
- Replayable
- Auditable
- Phase-driven

If growth reduces clarity,
growth must stop.

---

# 16. Ultimate Rule

If a change cannot be explained in:

- One diagram
- One paragraph
- One iteration trace

It does not belong in `rx`.

---

# Enforcement

Agents are expected to self-police against drift.

If architecture is violated,
the correct action is rollback or redesign,
not patching around the violation.

