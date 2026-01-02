# Contracts and Schemas

## Principle

Contracts are shared across renderer, host, and engines. **No ad-hoc DTOs** or duplicate payload
shapes in feature code.

---

## Where contracts live

- **TypeScript (renderer/adapters):** `app/AideonDesktop/src/dtos` is the canonical renderer-side
  contract surface.
- **Rust (host/engines):** DTOs live in the host and engine crates and are exposed via typed IPC and
  trait interfaces.

---

## Synchronization model

- DTOs are mirrored manually with **contract tests** in both stacks.
- Rust structs define field names and casing; TS mirrors must match exactly.
- Error envelopes are structured and stable; changes require tests + doc updates.

---

## How to change a contract

1. Update the Rust DTOs in the relevant host/engine crate.
2. Mirror the shape in `app/AideonDesktop/src/dtos`.
3. Update IPC handlers and adapters.
4. Extend contract tests (Rust + TypeScript).
5. Update the affected module `README.md`/`DESIGN.md`.

