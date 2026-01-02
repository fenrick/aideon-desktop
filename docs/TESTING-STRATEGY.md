# Testing Strategy

## Purpose

Describe how we test Aideon Suite across renderer, host, and engine crates. Module-specific detail
lives in module `README.md`/`DESIGN.md` docs.

---

## Coverage targets

- TS/React: >= 80% lines/branches/functions/statements on new code.
- Rust: host >= 80%, engines >= 90% where applicable.

Coverage failures are hard gates in CI.

---

## Required layers

- **Rust unit + integration tests** (engines + host)
- **TS/React component and adapter tests**
- **Contract tests** for DTO parity
- **E2E smoke** for critical flows (where configured)

---

## Common commands

- `pnpm run node:test`
- `pnpm run node:test:coverage`
- `pnpm run node:typecheck`
- `pnpm run host:lint && pnpm run host:check`
- `cargo test --all --all-targets`
- `pnpm run host:coverage` (requires `cargo-llvm-cov`)

---

## Rules

- Update tests whenever behavior or DTO shapes change.
- Prefer deterministic tests (fixed seeds for graph data).
- Validate boundary rules (no renderer HTTP, no ports in desktop mode).

