# Tool Evaluation: read_file

## Purpose
Ensure the `read_file` tool reads file contents verbatim and reports errors deterministically.

## Evaluation Steps
1. Invoke `read_file` on a small text file (`README.md`) and verify the returned `content` matches the file exactly.
2. Call the tool on a directory path to ensure it emits a structured error rather than panicking.
3. Request a non-existent file and confirm the tool reports the underlying I/O error deterministically.
4. Repeat a successful call to confirm results remain identical (same `content` and no side effects).

## Success Criteria
- `content` matches the UTF-8 file bytes.
- Errors are structured and actionable (no backtraces exposed to agent).
- Tool remains idempotent for repeated reads.
