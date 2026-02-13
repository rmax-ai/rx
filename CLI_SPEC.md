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
| `--resume <GOAL_ID>` | Resumes a previously started session identified by `GOAL_ID`. | `None` |
| `--list` | Lists all stored goals and their IDs with timestamps. | `false` |

## Environment Variables

| Variable | Description | Default |
| :--- | :--- | :--- |
| `OPENAI_API_KEY` | The API key for OpenAI. If not set, the agent defaults to using a `MockModel` for testing. | `None` |
| `OPENAI_MODEL` | The specific OpenAI model to use. | `gpt-5.1-codex-mini` |

## Files

| File | Description | Location |
| :--- | :--- | :--- |
| `LOOP_PROMPT.md` | The system prompt file used to initialize the agent's context. | Current working directory |
| `rx_state.db` | The SQLite database storing agent state and history. | System local data directory (e.g., `~/.local/share/rx_data/` on Linux/macOS) |

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
