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
| `--debug-log <PATH>` | Writes structured debug events to the supplied file (JSONL). The path may contain a `{goal_id}` placeholder that is substituted with the active goal ID. | `disabled` |
| `--list` | Lists all stored goals and their IDs with timestamps. | `false` |
| `--tool-verbose` | Prints tool inputs and outputs to stdout during execution. | `false` |
| `--model <NAME>` | Overrides the main agent model for this run. | config value, then `OPENAI_MODEL`, then `gpt-4o` |
| `--small-model <NAME>` | Overrides the small model for this run (auto-commit + goal slug generation). | config value (or `gpt-5-mini` when auto-commit enabled) |
| `--agent <NAME>` | Activates a named agent profile defined in `.rx/config.toml`, applying profile-specific defaults and optional model overrides. | none |

New sessions are assigned goal IDs in this format: `YYYYMMDD-HHMMSS-<goal-slug>`.

`<goal-slug>` is derived from the goal text. If `small_model` is configured and `OPENAI_API_KEY` is present, `rx` asks the small model to produce the slug and then sanitizes it.

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

### Override models for one run
```bash
rx --model gpt-5.2-codex --small-model gpt-5-mini "Refactor auth flow"
```

### List previous sessions
```bash
rx --list
```

### Resume a previous session
```bash
rx --resume 20231027-103000-refactor-auth-module
```

---

## Configuration File Specification

`rx` loads a local `.rx/config.toml` to set default CLI options.

### `.rx/config.toml` schema:

```toml
[cli_defaults]
max_iterations = 50         # Positive integer
auto_commit = false         # Boolean
small_model = ""            # String model name for commit messages and optional goal slug generation
resume = ""               # String goal ID (ignored in Phase 1)
debug_log = ""             # Path string (empty disables logging). Supports `{goal_id}` placeholder to embed the goal ID.
list = false                # Boolean
model_name = ""            # String model name for main agent
tool_verbose = false        # Boolean
```

Place this configuration file at the root of the workspace. Missing keys fall back to the CLI_SPEC.md defaults.

When `--auto-commit` is enabled and `small_model` is unset, commit messages default to the `gpt-5-mini` model.

`auto_commit_model` is accepted as a deprecated compatibility key and is used only when `small_model` is not set.

---

For more details on configuration and precedence rules, refer to `CONFIG_SPEC.md`.
