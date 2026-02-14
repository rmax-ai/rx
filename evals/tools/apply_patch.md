# Tool Evaluation: apply_patch

## Purpose

Validate the standalone `apply_patch` binary parses the documented patch envelope and applies file edits deterministically with safe path constraints.

## Evaluation Steps

1. Build the target with `cargo build --bin apply_patch`.
2. In a temporary directory, run `apply_patch` with an `*** Add File:` operation and confirm the file is created with the expected content.
3. Apply an `*** Update File:` patch with one hunk (`@@` + context/remove/add lines) and verify the file is updated exactly once.
4. Apply an `*** Update File:` patch with `*** Move to:` and verify the destination file exists with patched content while the source path is removed.
5. Apply a `*** Delete File:` patch and verify the file is removed.
6. Run the tool with an absolute path (for example `*** Add File: /tmp/x`) and confirm it fails with a path validation error.
7. Run the tool with a parent traversal path (`../x`) and confirm it fails with a path validation error.
8. Run a patch with mismatched hunk context and verify it fails deterministically (no partial write to the target file).
9. Run the same valid patch twice: first run succeeds, second run fails in a predictable way (for example, add-file already exists or hunk no longer matches), demonstrating deterministic behavior.

## Success Criteria

- The binary accepts the patch envelope and file operation headers exactly as specified.
- `Add`, `Update`, `Move to`, and `Delete` operations modify filesystem state as expected.
- Absolute and traversal paths are rejected before any file mutation.
- Hunk matching failures produce deterministic errors and do not partially modify files.
- Re-running identical patches yields consistent, explainable outcomes.
