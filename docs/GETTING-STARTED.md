# Getting Started (Developer Setup)

## Purpose

Set up a local dev environment for Aideon Suite (desktop renderer + host + engines). This guide is
about reproducible setup and common workflows; architecture lives in `docs/DESIGN.md` and
`ARCHITECTURE-BOUNDARY.md`.

---

## Prerequisites

- Node.js 24
- pnpm 10 (via Corepack: `corepack enable`)
- Rust toolchain (via `rustup`)
- GitHub CLI (`gh`) only if you use the issue helpers

---

## 1) Clone and install

```bash
git clone https://github.com/fenrick/aideon-desktop.git
cd aideon-desktop

corepack enable
corepack prepare pnpm@10 --activate
pnpm install
```

---

## 2) Run in development

Use two terminals:

```bash
# Terminal A: renderer
pnpm run node:dev

# Terminal B: host (Tauri)
pnpm tauri dev
```

Notes:

- Dev builds may use a local dev server; packaged builds load local assets and do not require
  network ports.
- Desktop baseline security rules still apply: renderer calls host via typed IPC and must not do
  ad-hoc HTTP.

---

## 3) Quality checks

```bash
pnpm run node:lint
pnpm run node:typecheck
pnpm run node:test

pnpm run host:lint
pnpm run host:check
```

For the full script list, use `pnpm -w run` (and see root `package.json`).

---

## 4) Issues workflow (optional)

The repo includes helpers that use `gh` and can derive the repo slug from the git remote. You only
need `AIDEON_GH_REPO` if you’re working with a non-standard remote.

```bash
pnpm run issues:start 123
pnpm run issues:dod
pnpm run issues:mirror
```

---

## Troubleshooting

- `gh: not logged in`: run `gh auth login` or set `GH_TOKEN` in your environment.
- Rust toolchain missing: install rustup and run `rustup default stable`.
- Tauri build failures: confirm system dependencies for your OS; retry with `pnpm tauri dev`.
