# Tool Evaluation: write_file

## Purpose
Validate `write_file` safely writes content with explicit modes and guardrails against destructive edits.

## Evaluation Steps
1. Use `write_file` in `create` mode for a new file and confirm it succeeds only when the file was absent.
2. Attempt `create` mode on an existing file and ensure the tool rejects the operation.
3. Write new content with `overwrite` mode and verify the file’s bytes match the provided payload.
4. Trigger the guardrail that rejects drastically smaller replacements (without `force`) and confirm it suggests `force=true`.
5. Append content using `append` mode and check the file contains both original and appended sections.
6. Use `force=true` to bypass the size guardrail and validate the overwrite completes with the new content.

## Success Criteria
- Modes behave as documented without side effects.
- Guardrails block suspicious overwrites but allow intentional ones with `force`.
- File contents match requested writes for every mode.
- Tool never leaves partial files when errors occur.
