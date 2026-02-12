# ARCHITECTURE

`rx` follows a microkernel architecture.

The kernel is small.
Everything else is a plugin or adapter.

---

## Layer Model

```

+--------------------------+
|      Transport Layer     |
+--------------------------+
|        Kernel Core       |
+--------------------------+
|        Tool Runtime      |
+--------------------------+
|      State Backend       |
+--------------------------+
|   Environment Adapters   |
+--------------------------+

````

Each layer has strict responsibilities.

---

## 1. Kernel Core

The kernel owns:

- Autonomous reasoning loop
- Tool dispatch
- Iteration control
- Termination evaluation
- Event emission
- State coordination

The kernel does NOT own:

- Filesystem logic
- Shell execution details
- Network implementation
- Persistence engine details
- Transport protocols

The kernel depends only on traits/interfaces.

---

## 2. Tool Runtime

Tools are isolated execution units.

A tool:

- Performs side effects.
- Returns structured output.
- Does not mutate kernel state directly.

Minimal interface:

```rust
trait Tool {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    async fn execute(&self, input: ToolInput) -> ToolOutput;
}
````

Tools must be:

* Idempotent where possible
* Deterministic given same inputs
* Observable (inputs + outputs logged)

Kernel invokes tools through a registry.

---

## 3. State Backend

State is append-only and event-driven.

Kernel interacts with state through an abstraction:

```rust
trait StateStore {
    fn load(&self, goal_id: &GoalId) -> State;
    fn append_event(&self, event: Event);
}
```

Phase 1:

* In-memory store

Phase 2:

* SQLite-backed event log

Kernel must not assume storage implementation.

---

## 4. Transport Layer

Transport delivers goals to the kernel.

Examples:

* CLI
* HTTP API
* Background daemon
* Distributed worker

Transport responsibilities:

* Accept input goal
* Stream events
* Handle cancellation

Transport does NOT:

* Contain business logic
* Execute tools
* Modify state directly

---

## 5. Environment Adapters

Adapters wrap:

* Local filesystem
* Shell
* Network
* External services

They are consumed by tools.

Kernel never touches environment directly.

---

## Execution Flow

1. Transport submits goal.
2. Kernel loads state.
3. Kernel requests next action from model.
4. Tool invoked via registry.
5. Tool returns output.
6. Kernel appends event.
7. Termination evaluated.
8. Repeat.

---

## Invariants

* Kernel remains small.
* Tools contain side effects.
* State is append-only.
* Every action is logged.
* No hidden global state.
* No tight coupling between layers.

---

## Growth Rules

If a component:

* Requires environment access → it is a tool.
* Persists data → it is state backend.
* Decides next action → it is kernel.
* Exposes input/output interface → it is transport.

If the kernel grows large, responsibilities are leaking.

---

## Anti-Patterns

* Embedding tool logic in kernel.
* Calling shell directly from kernel.
* Letting prompt define architecture.
* Coupling storage to business logic.
* Over-designing distributed features before Phase 2.

---

## Architectural Goal

`rx` must:

* Run fully offline.
* Be restartable.
* Be checkpointable.
* Be explainable in one page.


