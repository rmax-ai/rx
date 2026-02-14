# Tool Evaluation: list_dir

## Purpose
Verify that `list_dir` reliably enumerates directory contents and reports structured metadata for each entry.

## Evaluation Steps
1. Invoke `list_dir` on the repository root and confirm it returns entries matching `ls`, including both files and directories.
2. Call the tool on a deep directory (e.g., `src/tools`) and ensure each entry includes a `name` and `kind` field with accurate values.
3. Execute `list_dir` on a missing directory and confirm it emits a structured error instead of panicking.
4. Re-run the same successful call to ensure results remain identical and deterministic.

## Success Criteria
- Each entry contains `name` and `kind`, and reflects the actual filesystem state.
- Errors include the underlying I/O failure without leaking unstructured stack traces.
- Tool results are reproducible for the same valid path.
