# Evaluations

This directory captures the manual, human-readable evaluations that keep `rx` honest to its microkernel contract. Each Markdown file describes
- the *purpose* of the check,
- the concrete *evaluation steps* you should follow, and
- the *success criteria* that prove the invariant holds.

The prompting system and kernel do not automatically run these files. Instead, they serve as living documentation for anyone who wants to run, verify, or add an evaluation.

An automated runner is available for repeatable checks:

```bash
./evals/run.sh
```

Use OpenAI-backed runs when desired:

```bash
./evals/run.sh --with-openai
```

`--with-openai` executes `rx` through `direnv exec .` and expects `OPENAI_API_KEY` to be available from `.envrc`.

## Layout

```
evals/
├── features/  # holistic scenarios that stretch the kernel loop or tool registry
└── tools/     # per-tool behavioral checklists
```

### Current evaluations

#### Features
- `tool-registry.md` – verify the kernel only dispatches a fixed set of tools via a centralized registry and that unknown tool requests fail gracefully.
- `kernel-loop.md` – exercise the autonomous loop to ensure each iteration observes state, invokes a tool, persists the event, and terminates only when intended.

#### Tools
- `exec.md` – exercise the command runner for stdout/stderr handling, timeout behaviour, rejection of forbidden commands, and deterministic error reporting.
- `bash.md` – exercise shell-script execution using a single `script` string, including stdout/stderr capture, exit-code reporting, and timeout behaviour.
- `apply_patch.md` – verify the standalone patch applier correctly parses Add/Update/Delete/Move operations, enforces relative-path safety, and fails deterministically on invalid hunks.
- `read_file.md`, `write_file.md`, `list_dir.md`, and `done.md` – confirm each filesystem helper follows its documented contract, including guardrails around destructive writes and deterministic errors.

## How to run an evaluation

1. **Pick the spec** you care about (feature or tool) and read its sections.
2. **Follow the evaluation steps** verbatim using the CLI, shell commands, and `rx` invocations hinted by the spec. Examples:
   - Run `cargo run -- "<goal>"` (e.g., `cargo run -- "inspect the logs"`) and observe the tool calls to exercise the kernel loop.
   - Invoke `cargo run -- "read the file README.md"` or similar to trigger the filesystem tools described in the tool evaluations.
   - Use `cargo run -- --debug-log logs/rx-debug.jsonl "<goal>"` if you need detailed event tracing. The kernel also persists events in `rx_state.db` if you need to inspect the append-only store.
3. **Check success criteria**: confirm logs contain the expected entries, guardrails respond as described, and the kernel never grows per-tool logic.

For a quick baseline, run `./evals/run.sh` first, then use the specs for additional manual depth.

Because the evaluations are verbalized checklists, `rg`, `find`, or other OS commands can help you verify conditions (e.g., ensuring the tool registry file mentions every supported tool). Use the instructions in each spec to determine which manual commands best exercise the behaviour.

## Adding a new evaluation

1. Choose the right subdirectory (`features/` for loop-wide invariants, `tools/` for individual helpers).
2. Name the file descriptively (e.g., `tool-name.md` or `feature-name.md`).
3. Follow the existing structure: `## Purpose`, `## Evaluation Steps`, `## Success Criteria`.
4. Keep the steps short, actionable, and verifiable without modifying the kernel.
5. Link to related code or doc sections when helpful so future readers know where to look.

## What gets recorded

During these evaluations the kernel keeps structured logs. Tool dispatches appear in `logs/` and `rx_state.db` (or the debug log you request). When checking evaluation success, look for the expected tool names, arguments, and termination events in those artifacts.

Evaluations are human-readable stability checks, not automated tests. Treat them as guidance for manual verification and low-level documentation of the guarantees `rx` must uphold.
