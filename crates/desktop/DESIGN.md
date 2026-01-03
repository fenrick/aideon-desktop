# Aideon Desktop & Host

## Unified Design Specification (Evergreen)

---

## 1. Purpose and positioning

Aideon Desktop is a **native, offline-first, extensible enterprise digital twin environment**.
It embeds multiple analytical and modelling engines (Praxis, Mneme, Metis, Chrona, etc.) inside a secure desktop shell, providing:

- Rich, interactive visual modelling
- Time-aware and scenario-based analysis
- Large-scale graph analytics
- Long-running background processing
- Distributed sync (without Git mental models)
- Strong governance, auditability, and explainability

The desktop wrapper is **not a thin UI shell**. It is a **first-class application runtime** responsible for lifecycle, security, orchestration, and user experience coherence.

---

## 2. High-level architecture

### 2.1 Process model (desktop mode)

Single OS process, multi-layered:

```
┌──────────────────────────────────────────────┐
│ Renderer (TypeScript / React / shadcn)       │
│ - UI, views, diagrams, matrices, workflows   │
│ - No DB, no filesystem, no network            │
└───────────────▲──────────────────────────────┘
                │ IPC (typed, capability-gated)
┌───────────────┴──────────────────────────────┐
│ Aideon Host (Rust / Tauri)                    │
│ - Windowing                                  │
│ - Security & permissions                     │
│ - Job orchestration                          │
│ - Event bus                                  │
│ - Workspace lifecycle                        │
│ - IPC façade                                 │
└───────────────▲──────────────────────────────┘
                │ Trait calls (in-process)
┌───────────────┴──────────────────────────────┐
│ Engines (Rust crates)                        │
│ - Mneme (storage & processing)               │
│ - Praxis (meaning & interaction)             │
│ - Metis (analytics)                          │
│ - Chrona (time/scenario)                     │
│ - Continuum / Sync (future)                  │
└──────────────────────────────────────────────┘
```

**Invariant:**
The renderer never directly touches storage, the filesystem, or the network.

---

## 3. Core modules (complete set)

### 3.1 Aideon Host (Rust, Tauri)

**Responsibilities**

- Application lifecycle
- Window management
- Security and capability enforcement
- Workspace management
- Job orchestration
- Event broadcasting
- IPC façade and DTO validation
- Engine coordination

**Non-responsibilities**

- Business logic
- Analytics logic
- Domain semantics
- Rendering

---

### 3.2 Renderer (TypeScript, React, shadcn)

**Responsibilities**

- User interaction
- Visualisation (diagrams, maps, matrices, catalogues)
- Artefact composition (pages, reports)
- State management
- UX workflows
- Accessibility and responsiveness

**Non-responsibilities**

- Persistence
- Graph traversal
- Analytics execution
- Background processing

---

### 3.3 Engine modules (extensible, not exhaustive)

| Module        | Role                                                      |
| ------------- | --------------------------------------------------------- |
| Mneme         | Storage engine, event log, materialised state, processing |
| Praxis        | Metamodel, task model, artefacts, rules                   |
| Metis         | Analytics (PageRank, diagnostics, ranking, risk)          |
| Chrona        | Time, scenario, plateaus, temporal slicing                |
| Continuum     | Distributed sync (future)                                 |
| Import/Export | Structured IO, replay, migration                          |

The host **does not assume which engines exist**. Engines are registered via traits.

---

## 4. Workspace model (critical)

### 4.1 Definition

A **workspace** is the fundamental unit of user work.

A workspace encapsulates:

- One Mneme store
- One Praxis metamodel configuration
- Scenarios, plateaus, artefacts
- Jobs and history
- Permissions and audit trail

### 4.2 Workspace lifecycle (host-owned)

Host APIs:

- `create_workspace`
- `open_workspace`
- `close_workspace`
- `delete_workspace`
- `list_workspaces`
- `backup_workspace`
- `restore_workspace`

Rules:

- Only one workspace is active per window
- Workspace switching is explicit
- Engines are scoped to the active workspace

### 4.3 Storage locations

Host defines per-OS storage roots:

- Windows: `%APPDATA%/Aideon/workspaces`
- macOS: `~/Library/Application Support/Aideon`
- Linux: `~/.local/share/aideon`

Renderer never sees paths.

---

## 5. Window and UX shell model

### 5.1 Required windows

| Window     | Purpose                   |
| ---------- | ------------------------- |
| Splash     | Startup gating            |
| Main       | Primary workspace         |
| Settings   | Preferences               |
| Status     | Health, jobs, diagnostics |
| About      | Versioning, licences      |
| Styleguide | UI dev reference          |

Each window has a **stable label** and route.

### 5.2 Splash gating (authoritative)

Startup completes only when:

- Host backend initialisation finished
- Renderer splash rendered and signalled readiness

Failure paths must surface:

- Clear error state
- Access to Status window

---

## 6. Renderer shell architecture

### 6.1 Shell layout (fixed contract)

```
┌────────────────────────────────────────┐
│ Header / Toolbar (global + workspace)  │
├──────────────┬─────────────────────────┤
│ Left Nav     │ Main Content             │
│ (workspace)  │ (artefacts, diagrams)   │
├──────────────┴───────────────┬─────────┤
│ Footer / Status               │ Inspector│
└──────────────────────────────┴─────────┘
```

Workspaces **fill slots**.
They never own the chrome.

---

### 6.2 Design system

- shadcn primitives wrapped by Aideon design system
- Themes via `next-themes`
- Corporate styling via tokens
- No direct Radix/shadcn imports in product screens

---

## 7. IPC architecture (explicit)

### 7.1 Command categories

| Category  | Examples                |
| --------- | ----------------------- |
| System    | windows, setup, version |
| Workspace | open, close, migrate    |
| Jobs      | start, status, cancel   |
| Praxis    | artefacts, tasks        |
| Mneme     | storage, export/import  |
| Metis     | analytics runs          |
| Chrona    | time/scenario queries   |

### 7.2 DTO rules

- JSON only
- CamelCase
- Versionable envelopes
- Explicit error mapping
- No positional args (single payload object)

---

## 8. Job system (mandatory)

### 8.1 Why it exists

Analytics, imports, projections, migrations **must not block UI**.

### 8.2 Job model

Each job has:

- `job_id`
- `kind`
- `workspace_id`
- `status`
- `progress`
- `started_at`, `ended_at`
- `result_ref` (optional)
- `error` (optional)

### 8.3 Host responsibilities

- Run jobs asynchronously
- Persist job metadata
- Emit progress events
- Support cancellation
- Recover jobs after restart (where possible)

### 8.4 Renderer UX

- Global job tray
- Per-workspace job view
- Progress bars
- Retry/re-run actions

---

## 9. Event bus (push model)

Host → Renderer events:

- Job updates
- Workspace changes
- Model changes
- Integrity warnings
- Analytics completion
- Sync events (future)

Renderer subscribes declaratively.
Polling is discouraged except for fallback.

---

## 10. State management (renderer)

### 10.1 Global store (required)

A single store (e.g. Zustand / Redux Toolkit) managing:

- App context (workspace, scenario, time)
- Shell state (panels, selection)
- Jobs
- Notifications
- Cached artefact results

### 10.2 Cache invalidation

Driven by:

- Host events
- Workspace switches
- Scenario/time changes

Never by ad-hoc `useEffect` chains.

---

## 11. Artefact rendering contract (summary)

Artefacts (views, matrices, maps, pages, reports) are:

- Declarative
- Schema-validated
- Executed by Praxis
- Rendered by renderer from specs

Renderer never infers meaning.

---

## 12. Performance constraints (non-negotiable)

- Virtualised tables always
- Diagram LOD and clustering
- Progressive rendering
- Streaming large results
- Web Workers for heavy transforms
- Explicit upper bounds on artefact size

---

## 13. Security model

### 13.1 Capability registry

Every IPC command declares:

- Category
- Privilege level
- Required capability

Default: **deny**.

### 13.2 CSP

- Strict in production
- No remote assets
- Dev exceptions only in dev builds

### 13.3 Filesystem

- Workspace-scoped access only
- No arbitrary path access from renderer

---

## 14. Testing strategy (full stack)

### 14.1 Rust

- Unit tests
- DTO serde tests
- Host integration tests
- Capability enforcement tests

### 14.2 TypeScript

- Adapter tests (IPC)
- Component tests
- Store logic tests

### 14.3 Contract tests

- Command list parity
- DTO schema parity
- Error envelope stability

### 14.4 E2E

- Launch → splash → main
- Open workspace
- Run real job
- Render real artefact
- No network dependency

---

## 15. Extensibility model

### 15.1 Workspace modules / plugins

Defined by manifest:

- ID, version
- Required host version
- IPC namespaces
- UI slots provided

### 15.2 Engine registration

Host loads engines via traits, not hard references.

---

## 16. Non-goals (explicit)

- No Git metaphors
- No embedded server mode in desktop
- No renderer DB access
- No free-form drawing detached from model

---

## 17. Definition of Done (wrapper)

The desktop wrapper is complete when:

- All user workflows run without blocking UI
- All background work is job-managed
- All state transitions are event-driven
- All IPC is typed, tested, and gated
- Workspaces are portable and recoverable
- UI remains responsive at scale
- Engines can evolve independently

---

## 18. Final statement

This design deliberately treats **Aideon Desktop as a platform**, not an app shell.

It creates:

- A stable contract between UI and engines
- A safe environment for powerful analytics
- A foundation for long-term extensibility
- A UX that can scale from solo architect to enterprise programme

This is the level of structure required to avoid re-architecture as Aideon grows.

## 19. IPC command catalogue (authoritative structure)

This section defines **how IPC is organised**, not the full list of commands (that would be generated).

### 19.1 Command namespace rules

All IPC commands MUST follow:

```
<domain>.<capability>.<action>
```

Examples:

- `system.window.open`
- `workspace.open`
- `jobs.start`
- `jobs.cancel`
- `praxis.artefact.execute`
- `praxis.task.apply`
- `mneme.export.snapshot`
- `mneme.import.replay`
- `metis.analytics.run`
- `chrona.time.slice`

This enables:

- permission scoping
- logging and tracing
- automated contract tests
- future remote execution (without renaming)

### 19.2 Command envelope (mandatory)

All commands accept **one argument**:

```json
{
  "requestId": "uuid",
  "payload": {
    /* command-specific */
  }
}
```

All responses return:

```json
{
  "requestId": "uuid",
  "status": "ok | error",
  "result": {
    /* on ok */
  },
  "error": {
    "code": "string",
    "message": "string",
    "details": {}
  }
}
```

Rules:

- `requestId` always round-trips
- errors are structured and stable
- no Rust error strings leaked raw

---

## 20. Host permission & capability registry (explicit)

### 20.1 Capability declaration (Rust)

Every IPC command MUST declare:

```rust
#[aideon_command(
  capability = "workspace.write",
  risk = "high",
  audit = true
)]
async fn workspace_open(...) { ... }
```

Attributes:

- `capability`: required permission
- `risk`: low | medium | high
- `audit`: whether execution is logged for compliance

### 20.2 Capability classes

| Capability        | Meaning              |
| ----------------- | -------------------- |
| system.read       | app metadata         |
| system.write      | window control       |
| workspace.read    | model inspection     |
| workspace.write   | model mutation       |
| workspace.admin   | migrations, deletes  |
| jobs.run          | background execution |
| filesystem.export | export data          |
| filesystem.import | import data          |
| diagnostics.read  | logs, health         |

Default policy:

- Renderer starts with **read-only**
- Write/admin capabilities require explicit enablement

### 20.3 User-visible prompts (future-ready)

For high-risk capabilities:

- host may prompt user (desktop-native dialog)
- approval can be:
  - one-time
  - per-session
  - remembered

This is optional in v1, but the registry must support it.

---

## 21. Workspace manifest (plugin-ready)

Workspaces and modules are described declaratively.

### 21.1 Workspace manifest schema

```json
{
  "id": "praxis",
  "name": "Praxis",
  "version": "1.0.0",
  "hostVersion": ">=1.0.0",
  "capabilities": ["workspace.read", "workspace.write", "jobs.run"],
  "ui": {
    "navigation": true,
    "toolbar": true,
    "inspector": true
  },
  "ipcNamespaces": ["praxis.*"]
}
```

### 21.2 Host enforcement

Host:

- validates manifest on registration
- rejects incompatible versions
- limits IPC access to declared namespaces

Renderer:

- loads workspace modules dynamically
- uses manifest to wire shell slots

---

## 22. Renderer state architecture (mandatory)

### 22.1 Global store slices

The renderer MUST implement a central store with at least:

**System slice**

- app version
- platform (win/mac/linux)
- theme
- window focus

**Workspace slice**

- active workspace id
- available workspaces
- open artefacts
- dirty state flags

**Context slice**

- valid time
- scenario
- layer (plan/actual)

**Jobs slice**

- running jobs
- completed jobs
- failures

**Artefact cache slice**

- artefactId → result
- cache timestamp
- invalidation reason

**Notification slice**

- warnings
- integrity alerts
- job completion notices

### 22.2 Invalidation rules

Artefact cache invalidation happens when:

- workspace changes
- scenario changes
- valid time changes
- host emits `model.changed`
- job affecting artefact completes

Never via implicit React re-renders.

---

## 23. Job lifecycle (state machine)

### 23.1 Job states

```
Pending → Running → Completed
              ↘
               Failed
              ↘
               Cancelled
```

### 23.2 Required job metadata

```json
{
  "jobId": "uuid",
  "kind": "analytics | import | export | migration | projection",
  "workspaceId": "uuid",
  "progress": {
    "percent": 42,
    "stage": "Computing PageRank"
  },
  "startedAt": "...",
  "endedAt": "...",
  "resultRef": "optional",
  "error": "optional"
}
```

### 23.3 Job recovery

On restart:

- host reloads job registry
- jobs marked:
  - `Completed`
  - `Failed`
  - `Interrupted` (user can retry)

Renderer shows interrupted jobs explicitly.

---

## 24. Host event bus (typed)

### 24.1 Event categories

| Event               | Payload         |
| ------------------- | --------------- |
| `workspace.opened`  | workspace id    |
| `workspace.closed`  | workspace id    |
| `model.changed`     | scope summary   |
| `job.updated`       | job metadata    |
| `job.completed`     | job result ref  |
| `integrity.warning` | rule + entities |
| `analytics.updated` | artefact ids    |

### 24.2 Delivery guarantees

- Best-effort delivery
- No blocking renderer
- Renderer must tolerate missed events (fallback polling allowed)

---

## 25. Data lifecycle & migration

### 25.1 Migration model

- Each workspace has a schema version
- On open:
  - host checks version
  - runs migrations as jobs
  - blocks workspace use until complete

### 25.2 Failure handling

If migration fails:

- workspace opens in read-only recovery mode
- status window shows diagnostics
- user can export raw data

---

## 26. Import / export semantics (desktop)

### 26.1 Export types

- Full workspace snapshot
- Event log (replayable)
- Artefact-only (definitions)
- Reports (PDF/CSV/JSON)

Exports:

- are jobs
- stream data
- include manifest metadata

### 26.2 Import types

- Restore snapshot
- Replay event log into empty workspace
- Merge artefacts only

Import conflicts are surfaced via:

- job findings
- post-import integrity report

---

## 27. Performance budgets (hard limits)

These are **contractual**, not aspirational.

| Area                             | Budget     |
| -------------------------------- | ---------- |
| UI frame time                    | <16ms      |
| IPC response (small)             | <50ms      |
| Artefact execution (interactive) | <500ms     |
| Background jobs                  | async only |
| Diagram nodes (interactive)      | ≤5,000     |
| Matrix rows/cols                 | ≤1,000     |
| Catalogue page size              | ≤200       |

Anything beyond must:

- paginate
- cluster
- summarise
- or move to background job

---

## 28. Diagram & visual rendering rules

### 28.1 Determinism

Given:

- same artefact
- same workspace
- same context

Renderer must produce:

- identical diagram structure
- stable layout (unless user overrides)

This is critical for:

- explainability
- screenshots
- review workflows

### 28.2 Custom diagrams

User-arranged diagrams:

- store layout metadata only
- never alter model semantics
- versioned like artefacts

---

## 29. Accessibility & enterprise readiness

### 29.1 Accessibility (minimum)

- Keyboard navigation everywhere
- No colour-only meaning
- Screen-reader friendly lists/catalogues

### 29.2 Auditability

- All write actions logged (actor, time, command)
- Decision artefacts linked to changes
- Exportable audit packs

---

## 30. Definition of completeness (revisited)

The Aideon Desktop + Host platform is **complete** when:

- All engines can run independently of UI
- UI never blocks on heavy work
- Jobs are visible, cancellable, recoverable
- Workspace lifecycle is explicit and safe
- IPC is typed, permissioned, audited
- State is event-driven, not incidental
- UX scales from hundreds to millions of elements
- New modules can be added without shell changes

---

## 31. Strategic closing

This design intentionally treats:

- **Tauri** as an application runtime, not just a wrapper
- **Rust** as the authority for safety, concurrency, and correctness
- **TypeScript/React** as a projection and interaction layer
- **The Desktop** as the _product_, not a distribution format

This is the level of structure required to support:

- serious enterprise modelling
- long-lived repositories
- analytical truth rather than diagrams
- future sync and collaboration

Continuing by **locking down the last set of concerns that usually remain implicit**, but which determine whether this becomes a durable platform or a brittle app.

What follows completes the spec at the **operational, organisational, and evolution** layers.

---

## 32. Engine orchestration model (host-level)

### 32.1 Engine registration contract

All engines register with the host via a common trait:

- engine ID
- version
- supported capabilities
- lifecycle hooks

Example lifecycle hooks (conceptual):

- `on_host_start`
- `on_workspace_open`
- `on_workspace_close`
- `on_job_start`
- `on_job_cancel`
- `on_host_shutdown`

**Rule:**
The host never calls engine internals directly; all interaction is via registered traits.

---

### 32.2 Engine isolation guarantees

Engines:

- must not spawn unmanaged threads
- must register background work via the job system
- must not access filesystem paths outside workspace root
- must not communicate with renderer directly

This ensures:

- predictable shutdown
- safe cancellation
- consistent diagnostics

---

## 33. Multi-engine coordination patterns

Some workflows span engines (e.g. Praxis → Mneme → Metis).

### 33.1 Orchestration responsibility

- Host owns orchestration
- Engines expose _capabilities_, not workflows

Example:

- “Run analytics” is a host job that:
  1. Requests projection from Mneme
  2. Executes algorithm via Metis
  3. Stores results back via Mneme
  4. Emits completion event

Renderer sees:

- one job
- one progress stream
- one result

---

### 33.2 Failure containment

If one engine fails:

- job fails cleanly
- other engines remain operational
- workspace remains usable (unless failure is critical)

Critical failures (schema corruption, unrecoverable migration) trigger:

- read-only mode
- mandatory user acknowledgement

---

## 34. Diagnostics, logging, and observability

### 34.1 Logging tiers

| Tier      | Audience                      |
| --------- | ----------------------------- |
| User      | Status window, human-readable |
| Operator  | Structured logs, warnings     |
| Developer | Debug logs, traces            |

Host normalises logs from all engines into:

- unified log stream
- severity levels
- correlation IDs (jobId, requestId)

---

### 34.2 Status window contract

The Status window must show:

- engine health
- active jobs
- recent failures
- migration state
- last successful backup/export
- version and build metadata

This window must remain usable even if:

- main workspace UI fails
- an engine is degraded

---

## 35. Versioning and compatibility strategy

### 35.1 Semantic versioning rules

- Host version: governs IPC compatibility
- Engine versions: independent, declared
- Workspace schema version: explicit and migratable

Breaking changes require:

- host major version bump
- explicit migration paths

---

### 35.2 Forward compatibility

Design rules:

- Renderer must tolerate unknown fields in responses
- Host must accept older DTOs where feasible
- Artefact schemas are versioned independently

This allows:

- gradual rollout
- safe upgrades
- mixed-version testing

---

## 36. Distribution and update model (desktop)

### 36.1 Build targets

- Windows (x64)
- macOS (arm64 + x64)
- Linux (AppImage or equivalent)

Each build includes:

- static renderer assets
- host binary
- embedded engine crates

---

### 36.2 Update strategy (future-safe)

Even if not implemented immediately, the design must allow:

- signed updates
- delta updates
- rollback on failure

Host must expose:

- version check
- update readiness state
- “restart required” signalling

Renderer handles:

- user consent
- progress display

---

## 37. Offline-first guarantees

The desktop application must function fully offline for:

- modelling
- analytics
- artefact rendering
- scenario planning
- exports

Only excluded:

- optional sync
- update checks (deferred)

**Rule:**
No feature may silently degrade due to lack of connectivity.

---

## 38. Sync readiness (without committing to implementation)

The design must not block future sync.

Preparation points already baked in:

- event-log-based storage (Mneme)
- job-based background processing
- conflict surfacing at domain level
- workspace isolation
- explicit audit trails

When sync arrives:

- it is an engine
- registered like others
- coordinated by host
- surfaced as jobs + events

No UI rewrite required.

---

## 39. Organisational scaling considerations

This spec assumes:

- multiple teams working in parallel
- engines evolving independently
- UI teams not blocked on backend internals
- long-lived data and artefacts

Therefore:

- contracts matter more than code structure
- tests are part of the interface
- specs are evergreen, not snapshots

---

## 40. Anti-patterns (explicitly forbidden)

The following must never appear:

- Renderer importing database or engine crates
- Engines emitting UI events directly
- Long-running work on IPC handlers
- Hidden background threads
- Unbounded graph queries from UI actions
- Free-form editing that bypasses Praxis tasks
- Git metaphors leaking into UX

If these appear, it is a design violation, not a refactor opportunity.

---

## 41. Architectural decision record (ADR) expectations

Major changes require:

- ADR with context, decision, consequences
- Linked to code and migrations
- Stored alongside the repository

This avoids silent erosion of the architecture.

---

## 42. Final system invariants (non-negotiable)

1. **The renderer is disposable**
   Losing UI state must never lose model state.

2. **The host is authoritative**
   All side effects flow through it.

3. **Engines are replaceable**
   Their contracts matter more than their internals.

4. **Time and scenario are first-class**
   No “current state only” shortcuts.

5. **Analytics must be explainable**
   If you can’t explain it, you can’t trust it.

6. **Users never manage branches**
   They manage scenarios and decisions.

---

## 43. Final completeness statement

With Sections **1–43**, the Aideon Desktop & Host design now covers:

- runtime architecture
- security and permissions
- UX shell and interaction
- background processing
- extensibility and evolution
- testing and quality gates
- enterprise readiness
- offline-first operation
- future sync readiness

---

## 44. Aideon Host (Rust/Tauri) detailed design

This section makes the host buildable and maintainable as the suite expands beyond the currently documented modules.

### 44.1 Host crate responsibilities (hard boundaries)

The host is responsible for:

- **Windowing**: create, show/hide, focus, centre, size, vibrancy, menus.
- **Security**: CSP, Tauri capabilities, command allowlists, filesystem boundaries.
- **Workspace lifecycle**: open/close/create/list/backup/restore (even if initially stubbed).
- **IPC façade**: typed commands only, DTO validation, stable error envelopes.
- **Job orchestration**: run background work across engines, progress, cancellation, persistence.
- **Event distribution**: push events to renderer (job updates, model changed, sync status).
- **Engine wiring**: register trait objects and route commands/jobs to engines.

The host is **not** responsible for:

- domain logic
- analytics logic
- traversal logic
- rendering logic

This aligns with `ARCHITECTURE-BOUNDARY.md` where host “wraps engine traits” and enforces “no open TCP ports” in desktop mode.

---

### 44.2 Host module layout (authoritative)

`crates/desktop/src/` must be organised by responsibility, not by “whatever was added next”.

Required modules:

- `app.rs`
  Tauri builder, plugin init, invoke handler registration, managed state setup.

- `windows.rs`
  All window creation, labels, sizing, platform styling. No engine calls.

- `menu.rs`
  Native menu and accelerators. Emits events to renderer or calls host window commands.

- `setup.rs`
  Splash gating and backend initialisation. Owns setup state machine.

- `ipc/` (new folder; consolidates today’s command sprawl)
  - `ipc/mod.rs` – registry glue and shared DTO helpers
  - `ipc/system.rs` – version, environment, diagnostics toggles
  - `ipc/workspace.rs` – open/close/list/backup/restore
  - `ipc/jobs.rs` – start/cancel/list, progress subscriptions
  - `ipc/praxis.rs` – artefact execution + task application
  - `ipc/mneme.rs` – storage primitives + export/import + subscriptions
  - `ipc/chrona.rs` – time/scenario UX and temporal snapshots (where applicable)
  - `ipc/metis.rs` – analytics job entrypoints
  - `ipc/continuum.rs` – orchestration/scheduler entrypoints (when enabled)

- `health.rs`
  Host + engine health aggregation.

- `events.rs` (new)
  Host→renderer event bus wrappers and event schema.

- `jobs.rs` (new)
  Host job manager + persistence + cancellation token plumbing.

This does not require large refactors day one, but it is the target structure.

---

### 44.3 Command naming and compatibility with current code

Your current host command names are flat (e.g. `temporal_state_at`, `mneme_create_node`) and registered in `app.rs`.

Future-proof rule:

- **Host keeps existing command names as stable IDs.**
- **Renderer exposes namespaced methods via adapters.**

Example mapping:

- Adapter method: `chrona.time.stateAt()`
  Calls host command: `temporal_state_at`

This avoids breaking current code while giving you a scalable mental model for “all possible modules”.

---

### 44.4 Host state: managed singletons

Host must manage these in Tauri state:

- `SetupState` (already present)
- `WorkspaceManager` (new)
- `JobManager` (new)
- `EventBus` (new)
- `EngineRegistry` (new; trait objects for each engine)

None of these are accessed directly by renderer. Renderer gets them through commands/events only.

---

## 45. Window model and UI routing contract (detailed)

### 45.1 Window labels (stable IDs)

These labels must never change:

- `splash`
- `main`
- `settings`
- `status`
- `about`
- `styleguide`

(Your current `windows.rs` already follows this pattern.)

### 45.2 Route contract (renderer paths)

Routes must be static-export safe and stable:

- `/` main shell
- `/splash`
- `/settings`
- `/status`
- `/about`
- `/styleguide`

Host loads these via `WebviewUrl::App("route/")` or equivalent.

### 45.3 Platform-native styling

Host must set:

- Windows:
  - Mica (where supported)
  - correct window background to avoid “white flash”

- macOS:
  - titlebar style consistent with corporate app feel
  - traffic-light spacing correct for sidebar layouts

- Linux:
  - default decorations on main, off on splash

The renderer must assume **no** platform-specific CSS hacks are required to make the shell look correct.

---

## 46. Renderer (TypeScript/Next/shadcn) detailed design

This aligns with `app/AideonDesktop/DESIGN.md` and extends it to support more modules and long-lived complexity.

### 46.1 Renderer project structure (authoritative)

`app/AideonDesktop/src/` must contain:

- `app/`
  Window-aware screens and routing glue (`app-screens.tsx` etc.)

- `design-system/`
  Aideon-wrapped shadcn primitives and tokens. Product screens never import raw shadcn.

- `shell/`
  `AideonDesktopShell` and slot composition.

- `workspaces/`
  Workspace modules registered in `registry.ts` and implementing `WorkspaceModule`.

- `adapters/`
  IPC adapters and contracts (`timegraph-ipc.ts` is the current exemplar).

- `state/` (new if not already present)
  Global store slices:
  - system
  - workspace
  - context (time/scenario/layer)
  - jobs
  - notifications
  - artefact cache

- `testing/` (or `tests/`)
  Test harnesses and golden fixtures.

---

### 46.2 Workspace module contract (expanded)

Each module (Praxis, Mneme, Metis, Chrona, Continuum, and future modules) must implement:

- `id`, `name`, `icon`
- `navigation(): ReactNode`
- `toolbar(): ReactNode`
- `content(): ReactNode`
- `inspector(): ReactNode`
- optional `onActivate(ctx)` and `onDeactivate()`
- optional `routes[]` for module-internal deep links
- optional `capabilities[]` (for UI gating)

The shell remains constant. Modules are additive.

---

### 46.3 Theme system (corporate-grade)

Required behaviours:

- global light/dark/system
- colour theme selection
- persistence (local storage) with safe fallback in non-Tauri preview
- tokens applied at root with `data-*` attributes
- no module-specific themes

This keeps the product coherent as modules proliferate.

---

## 47. Host ↔ Renderer interaction model (beyond basic invoke)

You already have request/response IPC for commands. You now need first-class **events**.

### 47.1 Event bus (host → renderer)

Host emits events for:

- `setup.backend_ready`
- `setup.frontend_ready_ack`
- `workspace.opened` / `workspace.closed`
- `jobs.updated` (progress)
- `jobs.completed`
- `model.changed` (with scope)
- `integrity.updated`
- `analytics.updated`
- `sync.updated` (future)

Renderer subscribes via a single adapter (e.g. `adapters/events-ipc.ts`).

Rule:

- UI must not poll for job progress or model updates unless event subscription fails.

---

## 48. Job system (host-owned) build-ready specification

Your repo already hints at job concepts (`mneme_list_jobs`, failed jobs, triggers). This section standardises it at the host level.

### 48.1 Job API surface (stable)

Host must expose commands:

- `jobs_list`
- `jobs_get`
- `jobs_cancel`
- `jobs_subscribe` / `jobs_unsubscribe` (or event-driven only)

Engines must not expose their own ad hoc job tracking to the renderer.

### 48.2 Job execution rule

If an operation can exceed ~200–500ms or touches disk heavily, it is a job:

- imports (ops/snapshot stream)
- exports
- schema compile/rebuild
- integrity refresh
- analytics projection refresh
- PageRank computation
- compaction/retention
- sync (future)

### 48.3 Job progress contract

Progress must be:

- monotonic percent (0–100) when possible
- stage name (human-readable)
- optional metrics (rows processed, bytes, nodes/edges)

Renderer uses this to render:

- job tray
- status window diagnostics
- “busy but alive” UX

---

## 49. Workspace lifecycle and data storage contract (desktop wrapper level)

Even if Mneme owns schema and persistence, the **wrapper** must define how a user experiences “a workspace”.

### 49.1 Required UX flows

- First run:
  - create workspace
  - or open existing workspace file

- Open workspace:
  - show migration job progress if required

- Close workspace:
  - cleanly stop subscriptions and background workers

- Backup workspace:
  - export snapshot

- Restore workspace:
  - import snapshot into new workspace id

### 49.2 Host authority

Only host may decide:

- workspace root paths
- locks / concurrency
- migration order
- recovery mode

Renderer never sees filesystem paths.

---

## 50. Testing regime (explicit and aligned to your repo)

Your repo already documents the testing strategy. This section turns it into wrapper-specific requirements.

### 50.1 Rust tests (host)

Must include:

- **Invoke payload casing tests** (already present pattern)
- **CSP and window config tests** (already present pattern)
- **Command registration test**:
  - ensure command list in `app.rs` matches the expected registry
  - fails when commands are accidentally removed/renamed

### 50.2 TypeScript tests (renderer)

Must include:

- Adapter tests that mock `invoke` and assert:
  - correct command name
  - correct payload shape
  - correct response mapping

- Store slice tests (context changes invalidating caches)
- Shell composition tests (workspace slot contract)

### 50.3 Cross-boundary contract tests (missing today; required)

Add a generated artefact in CI:

- Rust produces a machine-readable command+DTO manifest
- TS consumes it to:
  - validate adapter mappings exist
  - validate DTO shape expectations

This prevents drift when “all possible modules” expand the command surface.

### 50.4 E2E tests (desktop)

E2E must run the real packaged (or dev) Tauri app and verify:

- splash gating works
- main window shows
- settings window opens from menu
- at least one real IPC call works (e.g., `temporal_state_at`)
- status window shows job list

E2E must be deterministic:

- use a golden workspace fixture
- freeze time where required
- disable network access

---

## 51. Extending to “all possible modules” (suite-level scaling rules)

This is the key part of your ask: the wrapper must scale past today’s modules.

### 51.1 Module categories

The wrapper must support modules of these categories, without changing the shell:

1. **Modelling & authoring**

- Praxis (tasks, views, artefacts)
- Metamodel editors (advanced)
- Diagram/layout editors

2. **Persistence & governance**

- Mneme management (schema, projections, retention)
- Audit packs, evidence views
- Policy management (layer precedence, validity rules)

3. **Analytics & reasoning**

- Metis (centrality, impact, risk, cost)
- “Explain” tooling for analytics provenance
- What-if comparisons and deltas

4. **Time and scenario**

- Chrona (timeline UI, time slicing, plateaus)
- Scenario management and promotion workflows

5. **Orchestration & automation**

- Continuum (schedulers, triggers, automations)
- Background refresh policies

6. **Sync & collaboration (future)**

- Continuum sync engine
- Conflict surfacing + resolution workflows
- Federated workspaces

7. **Administration**

- Settings, preferences, diagnostics
- Import/export tooling
- Plugin manager (if you allow external plugins)

### 51.2 Wrapper invariants for all modules

Every module must:

- declare its UI slots
- declare the IPC namespaces it uses (via adapters)
- route all heavy work through jobs
- publish meaningful events for UI updates
- conform to the design system and theme tokens
- avoid owning chrome

---

## 52. “What to do next” to make this spec executable in your repo

These are the lowest-risk, highest-leverage next steps that align with your existing structure:

1. **Introduce an event adapter**

- host emits job+model events
- renderer subscribes in one place

2. **Introduce a host JobManager wrapper**

- even if engines internally manage jobs today, unify the view for renderer

3. **Add command manifest generation**

- small Rust build step that outputs JSON manifest of commands/DTOs
- TS test consumes it and validates adapters

4. **Formalise workspace lifecycle stubs**

- even if the underlying storage is “single workspace today”, the host surface should look like multi-workspace now
