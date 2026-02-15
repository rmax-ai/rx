# IMPLEMENT_RX_DURABLE_PLAN

Status: Phase 2 — Durable Execution
Owner: rx
Objective: Make execution restartable and replayable using SQLite.

This plan is executable by an autonomous coding agent.

Rules:
- Work sequentially.
- Do not skip tasks.
- Commit after each task.
- Do not introduce features outside Phase 2 scope.
- Preserve architecture invariants.

---

# PHASE 2 — DURABLE EXECUTION (SQLITE)

Goal:
A restartable agent that:
- Persists events to SQLite
- Can list active goals
- Can resume execution from a goal ID
- Reconstructs state from event history

---

## TASK 1 — Add Database Dependencies

[ ] Add dependencies to Cargo.toml:
    - rusqlite (bundled)
    - dirs
[ ] Run `cargo check` to verify.

Commit message:
"Add rusqlite and dirs dependencies"

---

## TASK 2 — Implement SqliteStateStore

[ ] Create `src/state/sqlite.rs` (or similar, refactor state.rs if needed)
[ ] Define `SqliteStateStore` struct
[ ] Implement `new(path: PathBuf)`
    - Initialize DB if not exists
    - Create `events` table (id, goal_id, type, payload, timestamp)
[ ] Implement `StateStore` trait:
    - `append_event`: Insert into DB
    - `load`: Select from DB by goal_id, ordered by timestamp/id

Commit message:
"Implement SqliteStateStore with basic schema"

---

## TASK 3 — Integrate SqliteStore into Main

[ ] Modify `main.rs`:
    - Determine data directory (e.g., `~/.local/share/rx` or `rx_data`)
    - Initialize `SqliteStateStore` instead of `InMemoryStateStore`
    - (Optional) Keep InMemory for testing if needed, or replace entirely.

Commit message:
"Switch kernel to use SqliteStateStore"

---

## TASK 4 — Implement Resume CLI

[ ] Add `--resume <goal_id>` flag to `main.rs` argument parsing.
[ ] Logic change:
    - If `--resume` is passed:
        - Do NOT generate new goal ID.
        - Do NOT prompt for goal text (unless goal event is missing?).
        - Load events from store.
        - If no events found, error out.
    - Else (new goal):
        - Generate new ID.
        - Append initial goal event.

Commit message:
"Add --resume CLI support"

---

## TASK 5 — Goal Listing (Usability)

[ ] Add `list` command or `--list` flag.
[ ] Implement `list_goals` in `StateStore` (or specific method on SqliteStore).
[ ] Display recent goals with timestamps and status (if inferable).

Commit message:
"Add goal listing capability"

---

## TASK 6 — Validation Pass

[ ] Run a goal: `rx "echo hello"`
[ ] Verify `.db` file creation.
[ ] Run `rx --list` -> see the goal.
[ ] Run `rx --resume <id>` -> Agent should see previous events and determine it is done (or continue if not).
[ ] Kill agent mid-task (simulated), resume, verify continuity.

Commit message:
"Phase 2 validation complete"

---

# DEFINITION OF DONE (PHASE 2)

The system is complete when:
- Events are persisted to SQLite.
- Agent can be stopped and resumed.
- Resume restores full context.
- No duplicate events on resume (unless re-executed).
- Architecture remains clean (Kernel doesn't know about SQLite).

---

# OUT OF SCOPE

- Distributed locking
- Migration system (simple init for now)
- Complex queries
- External database server
