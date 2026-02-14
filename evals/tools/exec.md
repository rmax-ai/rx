# Tool Evaluation: exec

## Purpose
Ensure the `exec` tool reliably runs system commands without a shell, reports stdout/stderr, and enforces timeouts.

## Evaluation Steps
1. Invoke `exec` with a simple command like `echo` and verify the response includes the expected stdout, empty stderr, and a success indicator.
2. Run a command that writes to stderr (e.g., `bash -c '>&2 echo err'`) to confirm stderr is captured and returned separately from stdout.
3. Execute a long-running command with `timeout_seconds` set below its duration to confirm `exec` returns a "timeout" error and `success: false`.
4. Attempt to run `apply_patch` to validate the explicit rejection path and error message.
5. Provide a non-existent binary name to confirm `exec` surfaces the spawn error rather than panicking.

## Success Criteria
- Outputs are recorded verbatim as part of the tool response (stdout/stderr strings and exit code).
- Timeouts return the structured `error: "timeout"` payload.
- Unsupported commands are rejected with the documented error message.
- Spawn failures propagate as structured errors without crashing the kernel.
