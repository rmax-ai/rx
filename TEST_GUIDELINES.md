# Effective Test Guidelines

Guided by the microkernel constraints that keep `rx` deterministic, observable, and minimal, these guidelines help contributors design tests that surface regressions without enlarging the kernel or relying on brittle external state.

## Purpose

- Capture how we reason about correctness across the kernel, tools, and state backends.
- Reinforce the Phase 1 priorities: local tools, in-memory state, and a hardened loop.
- Provide a repeatable checklist so everyone validates the same invariants before a change ships.

## Guiding Principles

1. **Determinism first.** Tests must be reproducible: fixed goals, mocked tool outputs when necessary, and sealed iteration caps.
2. **Kernel minimalism.** Tests should verify the kernel’s decision-making, not replicate tool or environment logic.
3. **Event observability.** Every check must assert against structured logs or replayable event snapshots so regressions are traceable.
4. **Failure readback.** Simulate tool failures and network glitches (via mocks or harnesses) to ensure the kernel remains composable and does not panic.
5. **Replayability.** When possible, capture the sequence of iterations so the same test can be run against future memory-to-SQL state transitions.

## Test Design Checklist

| Step | Description |
| ---- | ----------- |
| Goal selection | Choose a single, clear goal to drive the kernel. Prefer short-lived scenarios (file writes, simple command runs). |
| Tool orchestration | Keep tool inputs explicit. Use canned outputs for `exec` when verifying failure handling. |
| Iteration cap | Assert the loop stops within the configured cap. If you expect more iterations, document why and why the cap was raised. |
| State assertions | After each run, inspect the last event or state summary (via logs or in-memory views) to confirm the kernel recorded progress. |
| Logging | Validate the JSONL output contains the expected iteration count, tool inputs/outputs, and termination reason. |
| Failure injection | Introduce deterministic failures (tool errors, invalid file paths, panic flags) and confirm the kernel logs them before terminating or retrying. |

## Scenario Examples

- **Happy path iteration:** Run a goal that writes a file, reads it back, and finishes with `done`. Verify the event log shows each tool call and the loop terminates cleanly.
- **Failure handling:** Force the `exec` tool to return a non-zero exit code, then confirm the kernel logs the error, increments the failure count, and either retries or aborts based on the configured threshold.
- **No progress detection:** Submit a goal where the model refuses to act. Ensure the kernel detects the lack of state change after several iterations and terminates with a clear reason.

## Logging and Observability

1. **Structured Logs:** Prefer the JSONL log entries for assertions they capture iteration number, action, tool name, inputs, outputs, and termination reason.
2. **Human-readable audit:** Supplement JSONL checks with sampled stdout logs when you need to confirm human-facing messages (e.g., termination summaries).
3. **Event replay:** Capture the sequence of event IDs whenever tests uncover nondeterministic behavior so future runs can replicate the exact trace.

## Running and Automating Tests

- Execute `cargo test` to validate unit and integration suites focused on kernel invariants.
- Use `cargo run -- "<goal>"` for scenario-based smoke tests that touch tool execution and logging.
- Wrap long-running tests in harnesses that stub external tools (see `src/tools` for mockable interfaces).
- When writing new tests, add context in `TestGuidelines.md` if they exercise previously unhandled failure modes.

## What to Assert

- Kernel loop increments iteration count exactly once per iteration.
- Each tool call is accompanied by a logged event. The event should record the tool name, inputs, and outputs.
- Termination reason is explicit (`done`, `max iterations`, `fatal tool error`, etc.) and logged.
- Failure tolerance counters (if added) are respected; repeated tool errors lead to the configured abort path.
- No hidden state leaks: repeated test runs should produce identical logs when seeded with the same goal and tool outputs.

## When to Update These Guidelines

Anytime testing requirements change — for example, during the Phase 2 durable execution work or when introducing distributed tooling — extend this document with:

1. New deterministic practices needed for the phase.
2. Additional harnesses or mock strategies developers must use.
3. Observability checks that map to new logging or persistence behavior.

Summaries of major guideline updates should also land in the CHANGELOG or iteration logs that explain why the testing approach expanded.
