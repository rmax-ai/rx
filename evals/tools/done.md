# Tool Evaluation: done

## Purpose
Ensure the `done` tool consistently signals completion with a structured reason and does not mutate kernel state beyond the intent to terminate.

## Evaluation Steps
1. Invoke `done` with a clear reason (e.g., "goal achieved") and confirm the tool responds with `status: "done"` and echoes the reason.
2. Observe kernel logs to ensure the final iteration is recorded and no extra actions occur after `done` returns.
3. Attempt to call `done` multiple times consecutively to verify the kernel ignores redundant completion signals or handles them gracefully.
4. Validate that calling `done` without executing other tools still results in a clean termination event.

## Success Criteria
- The response payload always includes the provided reason.
- Kernel terminates immediately after acknowledging `done` without performing further iterations.
- Calling `done` more than once does not corrupt state or produce inconsistent traces.
