# Tauri Client-Server Pivot Strategy

## Purpose

Explain how the Tauri-based desktop app pivots from local-only mode to client-server mode without
changing the React renderer: which seams to use, how adapters switch from local to remote, and what
security posture we maintain. This doc is for contributors touching server mode or remote adapters;
detailed command/trait definitions remain in `ARCHITECTURE-BOUNDARY.md` and module DESIGN docs.

## Bottom line

Moving to a **React (UX) + Rust (host/compute)** desktop app makes the client-server pivot easier
provided we keep the renderer speaking a **single, typed command surface**, and hide locality
(local vs remote) behind **Rust service adapters**. Tauri IPC already mirrors client-server message
passing (Commands for request/response, Events for pub/sub, Channels for streaming), so we can swap
local implementations for remote ones without touching the React UI.

## How this helps the transition (renderer stays stable)

**Design seam:** treat every UX action as a call through a small TypeScript port (renderer code
under `app/AideonDesktop/`) -> Tauri **Command** -> Rust service **trait** (port). We ship two
adapters:

- **LocalAdapter (default):** calls in-process Rust modules (Praxis, Mneme, Metis).
- **RemoteAdapter (server mode):** calls a server over HTTP/2 or WebSocket, then returns the same
  DTOs.

Because the renderer only knows about command names and DTOs, we can flip local <-> remote by
config without UI churn.

## API surface (commands as internal RPC)

- Keep the renderer API stable across local/remote.
- Commands map to **task APIs** (authoring) and **artefact execution** (views/catalogues/matrices/
  maps/reports).
- Time context (valid time + layer) and scenario are explicit parameters on every call.

## Streaming and long-running work

- For progress/results streaming inside the app, use **Channels** (ordered, back-pressured).
- Avoid large JSON blobs; stream Arrow bytes or chunked payloads for large artefacts.

## Network posture and permissions

- Networking is enabled only in **server mode**.
- Capabilities restrict which windows can invoke network-enabled commands.
- Keep desktop mode offline: no open TCP ports, no renderer HTTP.

## Process model and boundary enforcement

- Core process orchestrates IPC and routes all requests, centralizing auth, rate limits, and
  telemetry.
- Validate and normalize incoming messages before they reach domain services.

## Authn/z and secrets (server mode)

- Terminate OAuth/OIDC in Rust, store tokens in a secure store, expose only short-lived access to
  the renderer via Commands.
- Capabilities + permission sets keep this compartmentalized.

## Reference architecture (client <-> server, unchanged UI)

```text
React UI
  -> TypeScript port helpers (invoke)
       -> Tauri Command: Tasks/Artefacts/Analytics
            -> LocalAdapter (default): Rust modules
            -> RemoteAdapter (server): HTTP/2 or WS client
                 -> Channels/Events -> UI for progress/updates
```

- Switch via config/env: `mode=local|remote`.
- Same DTOs/SLOs, so the UX does not care where the work runs.
