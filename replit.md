# Aideon Praxis

## Overview

Aideon Praxis is a desktop application built with Tauri (Rust backend + React/TypeScript frontend). It provides a Praxis workspace for visualizing and managing scenarios, projects, and templates with graph-based, catalogue, matrix, and chart views.

## Project Structure

```
/
├── app/
│   └── AideonDesktop/       # React frontend application
│       ├── src/
│       │   ├── adapters/    # IPC and platform adapters
│       │   ├── aideon/      # Desktop shell + platform canvas primitives
│       │   ├── praxis/      # Praxis workspace (widgets, templates, time UI)
│       │   ├── design-system/ # UI components (shadcn/ui based)
│       │   ├── dtos/        # Data transfer objects/types
│       │   ├── hooks/       # Custom React hooks
│       │   ├── types/       # TypeScript types
│       │   ├── main.tsx     # Application entry point
│       │   ├── root.tsx     # Root component
│       │   └── styles.css   # Global styles
│       ├── package.json
│       └── vite.config.ts
├── crates/                   # Rust backend crates (Tauri)
│   ├── aideon_praxis/
│   ├── desktop/
│   ├── engine/
│   ├── chrona/
│   ├── metis/
│   ├── continuum/
│   └── mneme/
├── aideon_xtask/            # Build task runner (Rust)
├── tests/                   # Integration tests
└── tools/                   # Build and development tools
```

## Development Setup

### Requirements

- Node.js 24+
- pnpm (package manager)
- Rust toolchain (for Tauri backend, not needed for frontend-only development)

### Running the Frontend

The frontend can run in two modes:

- Local-first (default, matches `pnpm tauri dev`): `127.0.0.1:1420`
- Replit UX mode (external webview): `0.0.0.0:5000`

Replit UX mode runs on port 5000 with Vite:

```bash
cd app/AideonDesktop
pnpm run dev:replit
```

### Key Scripts

- `pnpm run dev` - Start local dev server (port 1420)
- `pnpm run dev:replit` - Start Replit dev server (port 5000)
- `pnpm run build` - Build for production
- `pnpm run typecheck` - Run TypeScript checks

## Architecture

### Frontend Stack

- React 18.3 with TypeScript (downgraded from 19 for Radix UI compatibility)
- Vite 7 for build tooling
- Tailwind CSS 4 for styling
- shadcn/ui components with Radix UI primitives
- @xyflow/react for graph visualization
- Recharts for charts

### Tauri Integration

When running as a desktop app, the frontend communicates with the Rust backend via Tauri's IPC. In browser mode (Replit preview), the app uses mock data providers for all backend calls.

The `isTauri()` function in `app/AideonDesktop/src/praxis/platform.ts` detects the runtime environment and falls back to mock data when not in Tauri.

## User Preferences

- Uses light color scheme by default
- Space Grotesk / Inter font family

## Recent Changes

- Dec 17, 2025: Fixed React 19 / Radix UI compatibility issue
  - Downgraded from React 19 to React 18.3.1 to fix "Maximum update depth exceeded" error
  - The issue was caused by React 19's ref callback cleanup feature being incompatible with Radix UI's compose-refs utility
  - Updated @types/react and @types/react-dom to matching 18.x versions
  - Added React deduplication in Vite config to prevent multiple React copies in monorepo structure
  - Added ErrorBoundary component for better error diagnostics
- Dec 17, 2025: Initial import to Replit environment
  - Configured Vite to run on port 5000 with 0.0.0.0 host
  - Enabled allowedHosts for Replit proxy support
  - Upgraded to Node.js 24 to meet project requirements
