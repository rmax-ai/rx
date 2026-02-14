# Local CLI Config Specification

This spec defines how `rx` can optionally load a local `config.toml` to supply default CLI parameters while preserving the Phase 1 mandate for a minimal, deterministic, local kernel.

## 1. Motivation & Purpose
- Reduce repetitive flag usage when running the agent locally (e.g., during development iterations or testing).
- Keep defaults versioned alongside the workspace so every contributor has the same baseline without editing shell scripts.
- Preserve determinism and visibility by making defaults explicit in a schema checked into the repository (or local workspace).

## 2. Requirements
1. **Supported CLI Parameters**: Only the flags listed in `CLI_SPEC.md` can be defaulted through the config: `--max-iterations`, `--auto-commit`, `--resume`, `--debug-log`, `--list`, and `--tool-verbose`. In addition, the config may specify `auto_commit_model` (used only when auto-commit is enabled) and `model_name` for model selection. Any future additions must be cleared with maintainers before adding to the schema.
2. **Location**: The file lives at `<workspace-root>/.rx/config.toml`. If `.rx/` does not exist yet, the agent should create parent directories before writing (for CLI tools that emit defaults). Reading only occurs from the workspace root where `rx` is invoked.
3. **Loading Precedence**: `rx` applies defaults in the following order:1) Hardcoded defaults from `CLI_SPEC.md`; 2) Values in the local config file; 3) Values passed explicitly on the CLI at runtime. CLI flags always override config values, even if they match the defaults.
4. **Validation**: The config parser ensures each declared field matches the expected type (e.g., `max_iterations` must be a positive integer, `auto_commit` a boolean). The config is rejected if `resume` is supplied while `Phase 1` constraints forbid resume support. The parser emits structured log events for invalid files and falls back to CLI defaults without panicking.
5. **Phase 1 Constraints**: Resume remains disabled per `ROADMAP.md` (Phase 1 prohibits resume). The config schema must document this constraint, and any `resume` key must be ignored or rejected with a warning until Phase 2 or later. There must be no new distributed or persistence logic introduced by supporting this config, keeping everything local and deterministic.

## 3. Format & Schema
### Schema
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
### Notes
- Keys are optional; missing keys fall back to the hardcoded `CLI_SPEC.md` defaults.
- Boolean values use TOML literal syntax (`true`/`false`).
- Paths can be relative or absolute; they are resolved the same way the CLI normally resolves them.
- `auto_commit_model` is only used when `auto_commit` is enabled; otherwise commit messages fall back to the default.
- Comments are allowed for documentation but will be ignored by the parser.
### Example
```toml
[cli_defaults]
max_iterations = 80
auto_commit = true
auto_commit_model = "gpt-4o"
debug_log = "logs/rx-debug.jsonl"
list = true
tool_verbose = true
```

## 4. CLI Behavior
1. On startup, `rx` checks for `.rx/config.toml` in the current working directory. If found, it parses the `[cli_defaults]` table before parsing runtime flags.
2. Values declared in the config override the built-in defaults but remain subordinate to explicit CLI flags. For example, `--max-iterations 120` overrides a config value of `80`.
3. Flags that do not accept arguments (like `--auto-commit`) inherit the config value unless the CLI flag is supplied (which toggles the behavior regardless of the config). Flags that are not provided and have no config entry use the built-in defaults.
4. If the config file contains unsupported keys, the loader emits a warning but otherwise ignores them.

## 5. Operational Considerations
1. **Missing File**: Absence of `.rx/config.toml` is normal—CLI defaults apply, and the agent logs a low-level info event stating no config was found.
2. **Invalid File**: Syntax or validation errors cause a structured log event describing the failure. `rx` proceeds using CLI defaults, ensuring deterministic behavior even when the config is broken.
3. **Overrides**: Applying CLI flags when defaults exist results in a log entry that records which defaults were overridden for auditability. This supports traceability and replay readiness.
4. **Logging & Determinism**: The loader logs success/failure deterministically, so repeated runs under the same workspace produce the same sequence of events (up to user-supplied flags).
5. **No Distributed Changes**: The feature remains entirely local; it only reads a file and applies overrides—no new networking, persistence, or event replay machinery is introduced.

## 6. Testing Plan
1. **Precedence**: Write unit/integration tests that start `rx` with no flags/config, with config only, with CLI flags only, and with both to verify the ordering (default < config < CLI). Tests should run in an isolated temp workspace to prove determinism.
2. **Parsing**: Cover valid and invalid TOML to ensure the parser accepts supported fields, rejects incorrect types, and tolerates unknown keys. Use a mock logger to assert that parsing failures signal an error without crashing.
3. **Phase 1 Guardrail**: Confirm that `resume` entries in configs are ignored or warn (but never enable resume) while the roadmap forces Phase 1 constraints.

## 7. Documentation Updates
- Update `README.md` (or whichever user-facing doc covers running `rx`) to mention the new `.rx/config.toml` location and precedence rules.
- Document the schema in `CLI_SPEC.md` under a new subsection linking to this spec.
- Mention the deterministic logging behavior in `ARCHITECTURE.md` or logging docs if they describe config loading events.

## 8. Implementation Plan
1. **Parsing/Loading Strategy**: Wire the loader behind the existing CLI bootstrap so `.rx/config.toml` is parsed immediately after workspace discovery but before flag processing. Reuse the current TOML parser (or extend it) to deserialize `[cli_defaults]` into the same struct used by `CLI_SPEC` defaults. Log success/failure deterministically and do not panic on parse errors—fall back to builtin defaults.
2. **CLI Integration & Precedence**: During argument parsing, merge values from the loader with hardcoded defaults, then let the existing CLI parser override them with runtime flags. Emit a structured log event when config values are overridden by explicit CLI arguments to keep audit trails intact.
3. **Validation/Logging & Guardrails**: Enforce type/constraint checks as described earlier, including positive integers for `max_iterations` and booleans for toggles. Reject or ignore `resume` entries while Phase 1 forbids resuming, logging a warning that references the roadmap constraint. All validation paths log deterministically without branching into distributed or async behavior.
4. **Directory/File Handling**: Check for `<workspace>/.rx/config.toml` relative to the CWD, creating `.rx/` (and parents) only when a CLI helper writes defaults but never when simply reading. Fail gracefully if the file is unparseable or missing. Treat user-supplied relative paths (e.g., for `debug_log`) the same way CLI normally resolves them.
5. **Tests**: Add targeted tests covering parsing/loading (valid/invalid TOML), precedence (default vs config vs CLI), logging (warning on invalid schema or overridden defaults), and the Phase 1 `resume` guardrail. Use temporary workspaces so runs remain deterministic and independent. Mock or capture logs where needed to assert deterministic events.
6. **Documentation Update Plan**: Extend `README.md`, `CLI_SPEC.md`, and the relevant architecture/logging docs to describe the config location, schema, precedence, and deterministic logging behavior. Reference the Phase 1 resume prohibition explicitly and cross-link back to `CONFIG_SPEC.md` so future maintainers understand the overall approach.
