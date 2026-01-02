# pnpm Command Guide

## Purpose

List the common pnpm scripts for **Aideon Suite** and show how they map to JS/TS and Rust tasks
across modules (with Praxis as the current focus). Use this as a reference when running builds,
tests, and CI pipelines.

## JS/TS (app + adapters)

- `pnpm run node:dev` — Start the Next.js dev server for the renderer.
- `pnpm tauri dev` — Run the Tauri host in dev mode (expects the Next dev server to be running).
- `pnpm run node:build` — Build the static Next.js export (`app/AideonDesktop/out`).
- `pnpm run node:lint` — ESLint on TS/JS sources.
- `pnpm run node:lint:fix` — ESLint with `--fix`.
- `pnpm run node:typecheck` — TypeScript project references build.
- `pnpm run node:test` — Unit tests (Vitest) for the app.
- `pnpm run node:test:coverage` — App tests with coverage reports.
- `pnpm run node:format` — Prettier write.
- `pnpm run node:format:check` — Prettier check.

## Rust workspace (host + domain crates)

- `pnpm run host:format` — `cargo fmt` across the Rust workspace.
- `pnpm run host:format:check` — format check only.
- `pnpm run host:lint` — `cargo clippy --all-targets -- -D warnings` for the Tauri host + crates.
- `pnpm run host:check` — `cargo check` sanity pass.
- `pnpm run host:ci` — Aggregated Rust pipeline (clean → fix → lint → check → format).

## Packaging

- `pnpm run tauri:build` — Builds the Tauri app bundle.

## CI Aggregate

- `pnpm run ci` — Runs a superset: app lint/type/tests + Rust lint/check pipeline.

## Notes

- Rust engines run in-process during dev. Remote/server adapters will follow the same contracts.
- For quality gates and coverage targets, see `docs/CODING-STANDARDS.MD`.
