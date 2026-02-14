# Feature Evaluation: Kernel Loop

## Purpose
Verify that the kernel drives the autonomous loop correctly:
- Observes current state.
- Requests the next action.
- Dispatches tools.
- Persists structured events.
- Honors iteration and termination rules.

## Evaluation Steps
1. Run a simple goal that requires two tool calls (e.g., `read_file`, `write_file`).
2. Confirm logs show each iteration, the tool call dispatched, and the event appended.
3. Inject a stopping condition (e.g., call `done`) and ensure the kernel terminates gracefully without extra iterations.
4. Confirm iteration limit enforcement by running with `--max-iterations 1` and verifying the goal aborts with a deterministic reason.

## Success Criteria
- Every iteration corresponds to a tool call in the logs.
- Termination only occurs via `done` or iteration cap.
- Structured events persist even when the goal is brief.
- No hidden state leaks outside the append-only store.
