# Contributing to Aideon Praxis

Thanks for your interest in contributing! This guide explains how we work, how to set up your
environment, and what we expect in issues and pull requests.

## Values

- **Time-first, graph-native:** keep the meta-model and `state_at()` semantics authoritative.
- **Commits define time:** follow the standards in `ARCHITECTURE-BOUNDARY.md#time-&-commit-model-—-authoring-standards` when modelling change history.
- **Local-first, cloud-ready:** do not break desktop mode; design for a clean switch to server.
- **Security by default:** safe IPC, PII redaction, least privilege.
- **Evidence & tests:** measurable SLOs; PRs come with tests/docs.

## Ways to contribute

- **Bug reports:** clear steps to reproduce, expected vs actual, logs/screenshots.
- **Feature requests:** problem statement, acceptance criteria, mockups if relevant.
- **Docs:** fix typos, improve examples, add diagrams/captions.
- **Code:** pick a labeled issue, discuss approach, then open a PR.

Labels to look for: `good first issue`, `priority/P1`, `area/*`, `module/*`. Milestones (M0–M6)
track staged delivery (see `ROADMAP.md`).

## Development setup

### Prerequisites

- Node.js 24, pnpm ≥ 9 (via Corepack), Rust (stable toolchain)
- Optional: Graphviz for diagram exports

### Install

pnpm install

### Run

#### Tauri host + renderer

```bash
# Terminal A — React UI
pnpm run node:dev

# Terminal B — Tauri host
pnpm tauri dev
```

### Test, Lint & Coverage

#### TS/JS

```bash
pnpm run node:test
pnpm run node:lint
pnpm run node:typecheck
```

#### Rust

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
```

Quality gates

- Coverage targets apply to both codebases: Lines ≥ 80%, Branches ≥ 80%, Functions ≥ 80% on new code.
- Verify locally:
  - App: `pnpm run node:test:coverage`
  - Rust crates: `cargo test --all --all-targets` (use coverage tooling when available)
- See `docs/CODING-STANDARDS.md` for full coding standards and CI rules.

## Repository structure

- Legacy Svelte renderer has been removed; React + Tauri desktop is now the only renderer.
- `crates/desktop` — Rust desktop host (Tauri) and IPC surface.
- `crates/{engine, chrona, metis, continuum, mneme, aideon_praxis}` — domain crates for graph/time/analytics, orchestration, persistence, and the facade.
- `app/AideonDesktop` — React/Tauri renderer (canvas, design system, adapters, DTOs).
- `docs/` — Architecture content, design docs, C4 diagrams.
- `scripts/` — Minimal tooling entrypoints (issues.py). Legacy node scripts removed.

- **Adapters are contracts.** Do not leak backend specifics into the renderer.
- **Worker traits stay typed.** No open TCP ports in desktop mode; remote adapters must preserve the same command surface.

## Branching, commits, PRs

### Branching

- `main`: protected, always releasable.
- Feature branches: `feat/<short-name>`; fixes: `fix/<short-name>`; chores/docs:
  `chore/<short-name>`, `docs/<short-name>`.

### Conventional Commits

- `feat: add PlanEvent confidence filter`
- `fix(chrona): handle Arrow payload >50MB`
- `chore(ci): cache pip and pnpm`
- `docs(c4): update system context`

### Pull Requests

- Link an issue and milestone.
- Describe problem, approach, trade-offs, and testing.
- Update docs/ROADMAP if APIs or modules change.
- Keep `ARCHITECTURE-BOUNDARY.md` accurate when touching host/renderer/worker boundaries.
- Add/adjust tests and SLO baselines where relevant.

### PR checklist

- [ ] Lint & tests pass (TS + Rust)
- [ ] No heading-level jumps in docs (markdownlint clean)
- [ ] Security considerations noted (IPC/PII if relevant)

## Architecture changes

When a change affects the **RPC boundary**, **adapters**, **time semantics**, or **security posture**,
update:

- `ARCHITECTURE-BOUNDARY.md`
- The relevant module `DESIGN.md`
- Any affected suite docs (`docs/DESIGN.md`, `docs/UX-DESIGN.md`)

## Issue hygiene

When raising an issue, include:

- **Context:** what problem/user need this solves
- **Acceptance criteria:** observable outcomes
- **Non-goals:** what is explicitly out of scope
- **Artifacts:** screenshots, logs, API examples

Use labels (`type/*`, `area/*`, `module/*`, `priority/*`) and assign the **milestone** (M0–M6).

## Code style

- **TypeScript:** ESLint + Prettier, strict TS config.
- **Docs & scripting:** When touching Python tooling in `scripts/`, follow Ruff + Black defaults; keep imports sorted.
- **Docs:** markdownlint (see `.markdownlint.json` if present).

## Security

- Never open worker TCP ports in desktop mode.
- Renderer calls Host via Tauri invoke (typed adapters). Desktop mode runs engines in-process. No sockets.
- Redact PII on exports by default.
- “No renderer HTTP” means no renderer-initiated backend/network calls in desktop mode; the dev
  toolchain may use a loopback dev server for HMR.
- Report vulnerabilities privately: <security@yourdomain.tld> (replace with project address).

## Releases

- Conventional Commits in PR title and commits (e.g., `feat(time): add plateau diff endpoint`).
- semantic-release manages versioning and CHANGELOG on merges to `main`.
- Package installers are built and uploaded on GitHub Releases; worker dists attached.

## License and CLA

- License: MIT — see `LICENSE`.
- CLA: not required at this time.

Thanks for helping build Aideon Praxis!
