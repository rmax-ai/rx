# Plan: Implement Bash Tool

The goal is to provide a `bash` tool that allows the agent to execute shell commands using a single script string, which is more intuitive and flexible than the existing `exec` tool.

## 1. Specification

### Tool Name
`bash`

### Parameters
- `script` (string, required): The shell script to execute.
- `timeout_seconds` (integer, optional, default: 30): Maximum execution time.

### Output
- `stdout` (string): The standard output of the script.
- `stderr` (string): The standard error of the script.
- `exit_code` (integer): The exit code of the shell.
- `success` (boolean): Whether the script exited with code 0.

### Implementation Detail
- The tool will run `/bin/bash -c "<script>"`.
- It will use `tokio::process::Command` for asynchronous execution.
- It will include a timeout mechanism similar to the `exec` tool.

## 2. Tasks

- [x] Create `src/tools/bash.rs` with the `BashTool` implementation.
- [x] Export `BashTool` in `src/tools/mod.rs`.
- [x] Register `BashTool` in `src/main.rs`.
- [ ] Verify implementation with a manual test or simulation.
