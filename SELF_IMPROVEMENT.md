# SELF_IMPROVEMENT

This document governs how `rx` improves itself.

Self-improvement must be deliberate, minimal, and architecture-preserving.

Improvement is not feature expansion.
Improvement is clarity, determinism, and robustness.

---

# 1. Improvement Objectives

`rx` must continuously improve in:

- Determinism
- Observability
- Replayability
- Failure handling
- Documentation accuracy
- Architectural boundary enforcement
- Development workflow clarity

Improvement must never increase uncontrolled complexity.

---

# 2. Self-Improvement Loop

When working on `rx` itself, the agent must:

1. Identify friction or weakness.
2. Confirm it violates architecture, determinism, or clarity.
3. Propose minimal corrective change.
4. Implement incrementally.
5. Update documentation.
6. Validate via simulation or replay.
7. Log reasoning.

No speculative expansion allowed.

---

# 3. Allowed Improvements

Agents MAY improve:

- Logging structure
- Error reporting
- Kernel clarity
- Trait boundaries
- Tool isolation
- Documentation precision
- Replay mechanisms
- Phase alignment
- Development scripts

Agents MUST NOT:

- Add large new subsystems without roadmap alignment.
- Introduce distributed complexity before Phase 2 is stable.
- Expand kernel responsibilities.
- Encode system invariants only in prompt.

---

# 4. Documentation Synchronization Rule

If code changes behavior:

- Update ARCHITECTURE.md if boundaries changed.
- Update KERNEL.md if loop semantics changed.
- Update ROADMAP.md if phase scope changed.
- Update CONTRACT.md if behavioral rules changed.

Documentation must reflect reality.

Outdated documentation must be corrected immediately.

---

# 5. Drift Detection

Drift occurs when:

- Kernel grows beyond intended scope.
- Tools begin coordinating each other.
- State becomes mutable without event log.
- Replay becomes unreliable.
- Architecture requires explanation beyond one page.

When drift is detected:
Refactor before adding features.

---

# 6. Simplification Mandate

At regular intervals, agents must:

- Identify redundant abstractions.
- Remove unnecessary indirection.
- Reduce code surface.
- Consolidate duplicated logic.
- Eliminate speculative scaffolding.

Reduction is progress.

---

# 7. Determinism Preservation

Any improvement must preserve:

- Step-by-step traceability.
- Structured event logging.
- Replay capability.
- Stable termination semantics.

If a feature reduces determinism, it must be rejected.

---

# 8. Phase Discipline

Before improving, confirm current phase.

Phase 1:
- Focus on correctness and logging.

Phase 2:
- Focus on persistence and resume safety.

Phase 3:
- Focus on decoupling and transport abstraction.

Improvements must align with phase priorities.

---

# 9. Testing for Improvement

Improvement must be validated via:

- Behavioral simulation.
- Deterministic replay.
- Failure injection.
- Iteration cap testing.

Avoid brittle micro-unit tests unless they protect invariants.

---

# 10. Process Improvement

Agents may improve development process by:

- Enhancing CLI ergonomics.
- Improving structured logs.
- Adding debugging commands.
- Improving iteration visibility.
- Improving commit discipline automation.

Process changes must not introduce hidden coupling.

---

# 11. Refactoring Rule

If a change:

- Adds complexity
- Increases coupling
- Makes replay harder
- Requires larger explanations

It must be redesigned.

Refactor first.
Expand later.

---

# 12. Self-Review Requirement

Before finalizing improvement, ask:

- Is the kernel still small?
- Are boundaries still strict?
- Is replay still possible?
- Is the system easier to reason about?
- Is documentation simpler?

If not, revert.

---

# 13. Long-Term Direction (Optional)

Self-improvement may eventually include:

- Snapshotting
- Forkable execution
- Deterministic replays
- Multi-agent arbitration
- State diff visualization

These must only occur after durability is proven stable.

---

# 14. Core Principle

`rx` improves by:

- Removing accidental complexity.
- Strengthening invariants.
- Increasing visibility.
- Preserving minimalism.

Self-improvement is architectural tightening,
not architectural expansion.

