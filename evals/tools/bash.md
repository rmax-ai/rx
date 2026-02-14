# Tool Evaluation: bash

## Purpose
Ensure the `bash` tool reliably executes shell script strings, captures stdout/stderr, and enforces timeouts deterministically.

## Evaluation Steps
1. Invoke `bash` with a simple script like `echo hello` and verify the response includes expected stdout, empty stderr, and `success: true`.
2. Run a script that writes to stderr (e.g., `echo err >&2`) to confirm stderr is captured separately from stdout.
3. Execute a failing script (e.g., `exit 7`) and verify `exit_code` is surfaced and `success` is `false`.
4. Execute a long-running script (e.g., `sleep 2`) with `timeout_seconds: 1` to confirm `bash` returns `error: "timeout"` and `success: false`.
5. Repeat the same successful script twice to confirm deterministic behavior and no hidden side effects.

## Success Criteria
- Tool response includes `stdout`, `stderr`, `exit_code`, and `success` for completed executions.
- Timeout path returns the structured payload with `error: "timeout"` and does not crash the kernel.
- Script execution behavior is stable across repeated invocations.
