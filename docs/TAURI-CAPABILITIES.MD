# Tauri Capabilities

## Purpose

Describe the capability model for the Tauri-based desktop host: which permissions and capability
files we rely on, how they are configured, and what defaults we enforce. This doc is a reference for
contributors changing host capabilities or security posture; broader host architecture lives in
`ARCHITECTURE-BOUNDARY.MD`.

Parent: #95

## Goals

- Preserve strict boundaries: Renderer <-> Host via preload IPC; Host <-> Worker via UDS/RPC.
- No network ports in desktop mode; deny-by-default for file system and process access.
- Keep a single config path so desktop/server builds do not fork.

## Capability Files (to be added)

- `fs-read-app-data` - read-only access to app cache/config dirs.
- `uds-ipc` - open UNIX domain sockets for worker RPC only.
- `clipboard` - optional, disabled by default.

These will be referenced from `crates/desktop/tauri.conf.json` once implemented.

## Security Defaults

- `allowlist`: minimal; commands must be explicitly exposed.
- `CSP`: strict; no remote HTTP in renderer.
- `deno/telemetry`: disabled; no external calls without allowlist.

## Next Steps

- Define capability TOML files under `crates/desktop/capabilities/`.
- Add a build-time check ensuring capabilities are referenced in `tauri.conf.json`.
- Unit tests for deny-by-default behavior in preload IPC bridge.
