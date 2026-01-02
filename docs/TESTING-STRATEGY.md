# Testing Strategy - Praxis Desktop (Rust + Tauri + React/TypeScript)

> Doc update checklist: if you touch a public API, a cross-module boundary, or introduce a pattern that should become standard, update this file and the relevant module `DESIGN.md`/`README.md` in the same PR. If behaviour changes, update tests and the docs together.

## Purpose

Describe how we test Aideon Suite, with a focus on the Praxis desktop module: which kinds of tests
we use (Rust, Tauri, React/TS), where they live, and how coverage and Definition of Done are applied
across layers. It complements `ARCHITECTURE-BOUNDARY.md` and `docs/CODING-STANDARDS.md` by focusing
specifically on testing practice.

**Goal:** Maintain at least **80% automated test coverage** across the codebase (Rust + TypeScript),
with tests that give real confidence in the behaviour of the digital twin engine and the desktop UI.
Hard gate: pipelines must fail when coverage drops below the thresholds described below.

This document explains:

- what kinds of tests we use,
- which tools to use,
- what must be tested in each layer (Rust, Tauri, React/TS),
- how we measure coverage,
- what the definition of done is for new code.

---

## 1. High-level architecture (for testing purposes)

Praxis Desktop has three main parts:

1. **Rust Engine**
   - Owns the digital twin: nodes, edges, metamodel, facts, scenarios, time context, etc.
   - Exposes functions the rest of the app calls.

2. **Tauri Backend**
   - Wraps the Rust engine in Tauri commands (IPC).
   - Handles desktop-specific concerns (filesystem, configuration, etc.).

3. **React/TypeScript Frontend**
   - Desktop GUI written in React + TypeScript, using shadcn/ui + Tailwind.
   - Contains:
     - canvas runtime (React Flow),
     - widgets (graph, catalogue, matrix, chart, etc.),
     - sidebar/control plane, views, state management.

We test each layer separately, plus full end-to-end (E2E) flows.

---

## 2. Testing layers and tools

We use four main kinds of tests:

1. **Rust unit tests**
   - Test pure functions and small modules in the engine.
   - Run with `cargo test`.

2. **Rust integration tests (including Tauri commands)**
   - Test how multiple Rust modules work together.
   - Test Tauri commands (engine + IPC) without the GUI.
   - Run with `cargo test` in appropriate test crates/folders.

3. **TypeScript/React unit & integration tests**
   - Test TS utilities, hooks, and React components.
   - Use a JS test runner (Vitest or Jest) + React Testing Library.

4. **End-to-end (E2E) tests**
   - Launch the Tauri desktop app and test real user flows.
   - Use a browser automation tool (e.g. Playwright).

**Baseline commands (run from repo root):**

- JS/TS unit + component tests: `pnpm run node:test`
- JS/TS coverage (fails <80%): `pnpm run node:test:coverage`
- JS/TS typecheck: `pnpm run node:typecheck`
- Rust (engine + host + crates): `cargo test --all --all-targets`
- Rust coverage (requires `cargo-llvm-cov` installed):
  `pnpm run host:coverage` (writes `coverage/rust.lcov` and fails on errors)

If the repo already has a preferred tool (e.g. Vitest vs Jest, Playwright vs something else), use
that. Otherwise default to: Rust built-in `cargo test`; Vitest + React Testing Library for TS/React;
Playwright for E2E.

---

## 3. Rust engine testing

### 3.1. What to test

Rust unit tests must cover:

- **Core domain logic**
  - Node/edge creation and deletion.
  - Metamodel rules (valid types, allowed relationships).
  - Scenario overlays, layer precedence, and time context resolution.
  - Time-based queries (what exists at time t).

- **Validation and error handling**
  - Invalid inputs (bad IDs, illegal relationships).
  - Boundary conditions (empty graphs, maximum sizes, etc.).

- **Conversion and mapping logic**
  - Serialisation/deserialisation of twin data.
  - Mapping internal models to view models (if done in Rust).

### 3.2. How to structure tests

- Use Rust's built-in test framework.

- For small modules, place tests inline in the same file with:

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_add_node_writes_fact() {
          // ...
      }
  }
  ```

- For larger subsystems, create integration tests under `tests/` in the Rust crate.

### 3.3. Coverage expectations for Rust

- **Rust domain/engine code:** target >= 90% line coverage.
- **Host/IPC crates:** >= 80% line coverage with explicit error-path assertions.
- **Command:** `pnpm run host:coverage` (fails if tests fail; coverage reports are artefacts for CI).

The overall project target is 80%, but we expect higher coverage in the engine than in UI code.

---

## 4. Tauri backend testing

### 4.1. What to test

Tauri backend tests must cover:

- **Tauri commands**
  - Each public command (e.g. `get_graph_view`, `apply_operations`, `list_scenarios`, etc.) should have integration tests that:
    - construct a minimal Tauri context if needed,
    - call the command directly,
    - assert on the Rust data returned or side effects.

- **Error and edge cases**
  - Missing or invalid arguments.
  - Engine errors propagated through Tauri commands.

### 4.2. What _not_ to test in Rust

- Do **not** attempt to assert on HTML/React components from Rust.
- Do **not** start the full GUI in Rust tests.

Tauri + Rust tests stop at the IPC boundary.

### 4.3. Coverage expectations for Tauri layer

- Aim for **>= 80% coverage** of the backend (commands, IPC handlers, context initialisation code),
  with particular attention to error cases and payload casing.

---

## 5. TypeScript/React testing

### 5.1. Tools (assumed default)

- **Vitest** (or Jest) as the test runner.
- **React Testing Library** for components.
- Optional: testing library for hooks.

### 5.2. What to test

#### 5.2.1. Pure TypeScript logic

Anything that doesn't depend on the DOM should be treated like normal business logic and tested thoroughly:

- View builders:
  - e.g. `buildGraphView`, `buildCatalogueView`, `buildMatrixView`.

- Data transformers, selectors, and sorting/filtering functions.
- Global state store logic (reducers, actions if using something like Zustand, Redux, or custom hooks).

These should be covered with straightforward unit tests.

#### 5.2.2. Global view state and hooks

The global view state (selection, filters, time cursor, scenario, template) is critical. Test:

- Initial state is correct.
- State transitions when:
  - user selects/deselects nodes,
  - filters change,
  - time cursor changes,
  - template changes.

Use hook testing helpers or small harness components to drive events and assert on final state.

#### 5.2.3. Components and widgets

For major components:

- **GraphWidget**
  - Given a `GraphViewModel` prop, renders a set of nodes/edges.
  - Clicking a node:
    - fires a selection callback OR
    - updates global selection store.

  - React Flow integration:
    - verify minimal expected behaviour (e.g. nodes appear, selection handler is called), without trying to exhaustively test React Flow library itself.

- **CatalogueWidget**
  - Given a catalogue view, renders rows and columns.
  - Row selection updates selection store or invokes selection callback.
  - Bulk selection, basic filtering (if implemented).

- **MatrixWidget**
  - Row/column rendering, cell interaction.
  - Ensure selection/edit events are fired correctly.

- **Sidebar/control components**
  - Template dropdowns, filters, scenario selectors, etc.
  - When a user interacts, the right state changes occur.

Focus on **behaviour and interaction**, not pixel-perfect visual output.

#### 5.2.4. API layer (`praxisApi`)

- Unit tests that, with a mocked Tauri bridge, verify:
  - correct commands are called with correct arguments,
  - errors are handled gracefully and propagated usefully to callers.

Note: do **not** depend on real Tauri in TS unit tests; mock it.

### 5.3. What not to test in TS

- Do not test third-party libraries themselves (React Flow, shadcn, etc.).
- Only test your integration with them:
  - that you pass the right props,
  - that you handle callbacks correctly.

### 5.4. Coverage expectations for TS/React

- **Hard threshold:** Vitest enforces >= 80% for lines/branches/functions/statements. `pnpm run node:test:coverage` fails if below.
- **TS logic and state management:** aim for >= 90% line coverage.
- **React components overall:** aim for >= 70% line coverage (containers can be harder; prioritise branching logic and IPC interactions).

Overall, the TS/React part of the codebase should average **>= 80% line coverage**, with higher coverage for adapters/hooks than for purely presentational pieces.

---

## 6. End-to-end (E2E) testing

### 6.1. Purpose

E2E tests validate that everything works together from a **user's point of view**:

- Tauri shell starts correctly.
- Rust engine + commands work over IPC.
- React UI renders and allows real interactions.
- Data travels from engine -> UI and back.

They are not for coverage; they are for **confidence in critical flows**.

### 6.2. Tools

Assuming **Playwright** as default:

- It launches the Tauri app, attaches to the window, and drives clicks/keystrokes.
- Tests are written in TypeScript.

### 6.3. Flows to cover

At minimum, E2E should cover:

1. **App startup**
   - Tauri app launches.
   - Main window renders sidebar + canvas.

2. **Loading a default canvas/template**
   - On first open, a default template loads.
   - Graph widget appears with some nodes/edges.
   - No errors in the console/log.

3. **Selection synchronisation**
   - Click a node on the canvas -> selection is reflected in the sidebar/canvas.
   - Click a row in a catalogue widget -> corresponding node is highlighted in the graph.

4. **Simple edit flow**
   - Create/import a small set of elements.
   - Check that they appear in:
     - catalogue widget,
     - graph widget.

   - Apply a simple change (e.g. rename node) and verify it persists.

5. **Scenario/time basic behaviour**
   - Switch scenario or adjust time cursor (once implemented).
   - Verify that views update accordingly.

E2E tests should be **stable** and **fast enough** to run in CI (they might be in a separate job).

### 6.4. Coverage expectations for E2E

We do not chase coverage percentage from E2E tests. Instead:

- We require that **all major critical flows** above are covered.
- E2E suite should be small and robust, not exhaustive.

---

## 7. Coverage measurement and thresholds

We target **>= 80% overall line coverage** across Rust and TypeScript.

In practice:

- **Rust engine + Tauri**
  - Use coverage tooling compatible with the Rust toolchain (e.g. `cargo` coverage tools).
  - Target >= 85-90% for core engine modules, >= 80% overall.

- **TypeScript/React**
  - Use the coverage reporting from Vitest/Jest.
  - Set thresholds (e.g. in config) so the test run fails if:
    - `lines` < 80%
    - `statements` < 80%
    - `branches` < 70% (branch coverage is harder; adjust if needed).

We can relax thresholds slightly for auto-generated code or trivial glue, but **any exception must be documented with a clear reason** (e.g. platform bootstrap code, configuration boilerplate).

---

## 8. Test data and fixtures

### 8.1. Principles

- Use **small, realistic graphs** and datasets for tests, not random junk.
- Prefer _named fixtures_ that are easy to understand (e.g. `simple_service_capability_graph`, `three_layer_hierarchy_with_pain_points`).

### 8.2. Implementation

- In Rust:
  - Create helper functions to build typical graphs and scenarios for tests.
  - Keep them in a shared `test_utils` or similar module.

- In TypeScript:
  - Create fixture builders for:
    - view models (graph, catalogue, matrix, chart),
    - global state initialisations.

- For E2E:
  - Provide a small sample project / dataset that is loaded in test mode.
  - Keep it deterministic and stable.

---

## 9. When and how to write tests (for coding agents and humans)

Whenever new code is written or changed:

1. **Rust engine / Tauri**
   - Any new function with non-trivial logic **must** have unit tests.
   - Any new Tauri command **must** have at least one integration test.

2. **TypeScript logic**
   - Any new function that transforms data or holds business rules **must** have unit tests.
   - Any change that modifies branching logic or error handling **must** adjust existing tests or add new ones.

3. **React components**
   - New components:
     - If they contain important interaction or state logic, write tests.
     - Pure presentational components may be lightly tested (or skipped), but keep them simple.

4. **E2E tests**
   - When you add a new **critical flow** or significantly alter an existing one:

- add or update an E2E test that covers the end-to-end behaviour.

### 9.1. Minimum expectations by change type

- **Engine changes (Rust)**: add/extend unit or integration tests in the touched crate. Example:
  `crates/praxis/tests/temporal_ops.rs` covers time-context read APIs over real facts.
- **Host/Tauri changes**: add or extend IPC/command tests in `crates/desktop/tests` (or
  crate-level `#[cfg(test)]` modules) to exercise payload shapes and error propagation.
- **Canvas/React changes**: add Vitest + Testing Library tests under `app/AideonDesktop/tests/praxis` that
  mock IPC adapters and assert on rendered output and callbacks.

### 9.2. Golden vertical (time-context read APIs)

Use this as a template when adding new features:

- **Engine**: `crates/praxis/tests/temporal_ops.rs` writes a node fact, asserts
  time-context read results (state_at/diff equivalents).
- **Host**: `crates/desktop/tests/state_at_payload_camelcase.rs` ensures payload casing
  matches IPC expectations.
- **React**: `app/AideonDesktop/tests/praxis/components/time-cursor-card.test.tsx` and
  `app/AideonDesktop/tests/praxis/time/use-temporal-panel.test.ts` show how to mock `praxisApi` and assert
  time-context UI reactions.

### 9.3. Definition of Done for a PR

A pull request (or automated change set) is **not done** unless:

- All relevant tests (Rust, TS, E2E where applicable) **pass**.
- Coverage report still meets:
  - overall >= 80%,
  - Rust domain modules and TS logic modules >= 85-90% where practical.

- New or changed behaviour is covered by appropriate tests:
  - engine logic by Rust tests,
  - UI logic by TS/React tests,
  - critical user flows by E2E tests where relevant.

"No tests, but it works on my machine" is **not acceptable**.

---

## 10. Summary for the coding agent

When you, as an automated coding agent, implement or change code:

1. Identify which layer you are working in:
   - Rust engine, Tauri backend, TS logic, React components, or E2E flow.

2. For each change:
   - Plan corresponding tests **before or alongside** implementation.
   - Keep tests small, focussed, and readable.

3. Always ensure:
   - `cargo test` passes for Rust.
   - `npm/yarn/pnpm test` passes for TS/React.
   - Coverage thresholds are not breached.
   - E2E tests still pass if your changes affect critical flows.

4. If you deliberately skip tests for a small utility (rare), add a short code comment explaining why.

The testing strategy is not optional - it is part of how we keep Praxis stable as an evergreen product while we evolve the twin, the canvas, and the UI over time.
