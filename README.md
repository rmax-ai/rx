# rx

Minimal autonomous systems agent with a microkernel architecture.

`rx` is not a chatbot.
It is a goal-directed execution engine capable of modifying files,
running commands, invoking tools, and operating locally or in distributed mode.

---

## Philosophy

- Kernel decides.
- Tools act.
- State persists.
- Transport delivers.

The kernel owns reasoning and iteration.
Tools own side effects.
State is append-only.
Transport is replaceable.

If a component grows large, it does not belong in the kernel.

---

## Architecture

```

+--------------------------+
|      Transport Layer     |  CLI | HTTP | Worker
+--------------------------+
|        Kernel Core       |  Loop | Dispatch | Control
+--------------------------+
|        Tool Runtime      |  exec | fs | net | custom
+--------------------------+
|      State Backend       |  memory | sqlite | future
+--------------------------+

````

The kernel:
- Executes the autonomous loop.
- Dispatches tool calls.
- Enforces iteration limits.
- Persists structured events.

The kernel does NOT:
- Implement filesystem logic.
- Execute shell commands directly.
- Handle networking details.
- Depend on specific persistence engines.

---

## Execution Model

Each iteration:

1. Observe current state.
2. Decide next action (LLM).
3. Invoke tool.
4. Persist event.
5. Evaluate termination.

The loop stops when:
- `done` tool is invoked.
- Iteration cap is reached.
- No progress is detected.
- A fatal error occurs.

---

## Minimal Tool Set (Phase 1)

- `exec(command)`
- `read_file(path)`
- `write_file(path, contents)`
- `list_dir(path)`
- `done(reason)`

Tools are stateless from the kernel’s perspective.

---

## Quick Start (Phase 1 Target)

```bash
cargo build
cargo run -- "create a file hello.txt with content hi"
````

Expected behavior:

* The agent iterates.
* Uses tools.
* Logs structured events.
* Terminates deterministically.

---

## Configuration with `config.toml`

`rx` supports the use of a `.rx/config.toml` file to provide default values for CLI options. This file can specify values such as the default OpenAI model, iteration limits, auto-commit behavior, and more. The default location is `<workspace-root>/.rx/config.toml`.

Example:

```toml
[cli_defaults]
max_iterations = 100
auto_commit = true
small_model = "gpt-5-mini"
debug_log = "logs/rx-debug__{goal_id}.jsonl"
list = true
model_name = "gpt-5.2-codex"
tool_verbose = true
```

New goals use IDs like `YYYYMMDD-HHMMSS-<goal-slug>` for easier discovery. The slug is derived from the goal text, and when `small_model` plus `OPENAI_API_KEY` are available, `rx` uses the small model to improve slug quality before sanitizing.

For one-off overrides, use explicit runtime flags:

```bash
rx --model gpt-5.2-codex --small-model gpt-5-mini "audit event flow"
```

---

## Effective Testing

We keep testing lightweight and deterministic. Follow [TEST_GUIDELINES.md](TEST_GUIDELINES.md) to craft repeatable tests that respect the kernel constraints and preserve observability.

---

---

## Current Phase

Phase 1 – Minimal Core

* In-memory state
* Local tools
* Hard iteration cap

---

## Non-Goals

* No UI
* No embeddings
* No vector databases
* No agent mesh
* No framework bloat

If `rx` cannot:

* Run offline
* Resume from disk (Phase 2)
* Be explained in one diagram

It is too complex.

---

## Design Constraint

`rx` must remain small, understandable, and composable.

Complexity is introduced only when forced by real constraints.
