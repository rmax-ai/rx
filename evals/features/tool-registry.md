# Feature Evaluation: Tool Registry

## Purpose
Confirm the kernel exposes the minimal tool set and dispatches calls through a central registry to keep tool ownership out of the prompt.

## Evaluation Steps
1. Inspect `src/kernel.rs` (or equivalent dispatch layer) to verify the registry lists `exec`, `read_file`, `write_file`, `list_dir`, and `done` with typed contracts.
2. Run a goal that exercises each tool and assert the kernel logs contain the tool name recorded as part of the dispatched action.
3. Trigger a request requiring an unknown tool and verify the kernel rejects it with a structured error rather than crashing.
4. Check that no additional logic exists in the kernel for any tool beyond dispatch and logging (ensuring tool logic stays in `src/tools`).

## Success Criteria
- All expected tool names are registered once and resolved through the same dispatch path.
- Tool ownership remains isolated in `src/tools/*`; the kernel never blossoms into per-tool behavior.
- Unknown tool references are handled deterministically via the registry error path.
- Logs trace each tool dispatch so replay accounts for which tool executed per iteration.
