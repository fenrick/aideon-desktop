# Aideon Host – Aideon Suite module

## Purpose

Aideon Host is the Tauri-based desktop host for Aideon Suite. It owns window management, OS
integration, capabilities, and the typed IPC surface that connects the React renderer to the
Rust engines.

## Responsibilities

- Configure Tauri windows, capabilities, and CSP for the desktop app.
- Expose typed commands that wrap Praxis Engine, Chrona, Metis, Continuum, and Mneme traits.
- Enforce security posture: no renderer HTTP, no open TCP ports in desktop mode, least-privilege FS.

## Relationships

- **Depends on:** Engine crates (Praxis Engine, Chrona Visualisation, Metis Analytics, Continuum Orchestrator, Mneme Core).
- **Used by:** Praxis Canvas and Praxis Desktop via IPC (`@tauri-apps/api` from the renderer side).

## Running and testing

- Dev (with Tauri CLI): `pnpm tauri dev`
- Rust checks (workspace): `pnpm run host:lint && pnpm run host:check`
- Rust tests (host only): `cargo test -p aideon_desktop`

See `docs/GETTING-STARTED.md` for host + renderer dev workflow.

## Design and architecture

Host/renderer boundaries, capabilities, and time-first engine contracts are described in
`ARCHITECTURE-BOUNDARY.md`. Host security and capabilities are further detailed in
`docs/TAURI-CAPABILITIES.md` and `docs/TAURI-CLIENT-SERVER-PIVOT.md`.

## Related global docs

- Architecture and layering: `ARCHITECTURE-BOUNDARY.md`
- Tauri capabilities and pivot: `docs/TAURI-CAPABILITIES.md`, `docs/TAURI-CLIENT-SERVER-PIVOT.md`
- Coding standards: `docs/CODING-STANDARDS.md`
- Testing strategy: `docs/TESTING-STRATEGY.md`
