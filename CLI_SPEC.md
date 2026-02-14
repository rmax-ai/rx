# CLI Specification for `rx`

This document details the command-line interface for the `rx` autonomous agent.

## Usage

```bash
rx [OPTIONS] [GOAL]...
```

## Arguments

| Argument | Description |
| :--- | :--- |
| `[GOAL]...` | The goal or task description for the agent to execute. Multiple words are joined by spaces. Required unless using `--list` or `--resume`. |

## Options

| Option | Description | Default |
| :--- | :--- | :--- |
| `--max-iterations <N>` | Sets the maximum number of iterations the agent is allowed to perform. | `50` |
| `--auto-commit` | Enables auto-commit mode. The agent will automatically commit changes to the state. | `false` |
| `--resume <GOAL_ID>` | [Ignored in Phase 1] Resumes a previously started session identified by `GOAL_ID`. | `None` |
| `--debug-log <PATH>` | Writes structured debug events to the supplied file (JSONL). | `disabled` |
| `--list` | Lists all stored goals and their IDs with timestamps. | `false` |
| `--tool-verbose` | Prints tool inputs and outputs to stdout during execution. | `false` |

## Environment Variables

| Variable | Description | Default |
| :--- | :--- | :--- |
| `OPENAI_API_KEY` | The API key for OpenAI. If not set, the agent defaults to using a `MockModel` for testing. | `None` |
| `OPENAI_MODEL` | The specific OpenAI model to use. | `gpt-4o` |

## Files

| File | Description | Location |
| :--- | :--- | :--- |
| `LOOP_PROMPT.md` | The system prompt file used to initialize the agent's context. | Current working directory |
| `rx_state.db` | The SQLite database storing agent state and history. | System local data directory (e.g., `~/.local/share/rx_data/` on Linux/macOS) |
| `config.toml` | File for loading default CLI parameter values. | `<workspace-root>/.rx/config.toml` |

## Examples

### Start a new task
```bash
rx "Refactor the authentication module to use JWT"
```

### Start a task with a higher iteration limit
```bash
rx --max-iterations 100 "Analyze the logs for error patterns"
```

### List previous sessions
```bash
rx --list
```

### Resume a previous session
```bash
rx --resume 20231027-103000
```

---

## Configuration File Specification

`rx` loads a local `.rx/config.toml` to set default CLI options.

### `.rx/config.toml` schema:

```toml
[cli_defaults]
max_iterations = 50         # Positive integer
auto_commit = false         # Boolean
auto_commit_model = ""       # String model name for commit messages
resume = ""               # String goal ID (ignored in Phase 1)
debug_log = ""             # Path string (empty disables logging)
list = false                # Boolean
model_name = ""            # String model name for main agent
tool_verbose = false        # Boolean
```

Place this configuration file at the root of the workspace. Missing keys fall back to the CLI_SPEC.md defaults.

When `--auto-commit` is enabled and `auto_commit_model` is unset, commit messages default to the `gpt-5-mini` model.

---

For more details on configuration and precedence rules, refer to `CONFIG_SPEC.md`.
