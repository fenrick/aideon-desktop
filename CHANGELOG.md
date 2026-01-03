# [0.2.0](https://github.com/fenrick/aideon-desktop/compare/v0.1.0...v0.2.0) (2025-12-17)

### Bug Fixes

- **app/dev:** use uv run with required deps for worker server\n\n- Add --with uvicorn/fastapi/pydantic for dev spawn to prevent ModuleNotFoundError during yarn dev\n- Packaged behavior unchanged ([670f78f](https://github.com/fenrick/aideon-desktop/commit/670f78f89d2bf96f1930ab966f61a3afec655e68))
- **app/main:** handle response error events and reject with Error instances; add test for inter-process RPC rejection ([ac23fd2](https://github.com/fenrick/aideon-desktop/commit/ac23fd27938c6925bb860c3eb7f29674d559a503))
- **app/splash:** remove unused seconds prop; use Geist Mono Local for loading copy ([3dfa529](https://github.com/fenrick/aideon-desktop/commit/3dfa529bf5569ab4cc8decc14d51ad338108f76c))
- **app:** type-safe IPC parsing with tests; satisfy ESLint rules; add parser unit tests ([2d57af4](https://github.com/fenrick/aideon-desktop/commit/2d57af438560d2ff3705058d6df89153754cab6d))
- **ci,issues,ts:** use worker:sync in CI; add issues split+dod; fix TS shim lint/typecheck (Refs [#95](https://github.com/fenrick/aideon-desktop/issues/95)) ([0c80901](https://github.com/fenrick/aideon-desktop/commit/0c80901d34f0e47f2a667fde65fa531c0f236e1f))
- **ci:** stabilize svelte typecheck pipeline ([05c8a5c](https://github.com/fenrick/aideon-desktop/commit/05c8a5cfc06683c54c223ecbe64de0efae3efe4b))
- **dev-server:** type middleware and format postbuild ([6c09cd5](https://github.com/fenrick/aideon-desktop/commit/6c09cd506efdd7966891f5d1a42c69138f5224e9))
- **format:** fix formatting ([223f9bf](https://github.com/fenrick/aideon-desktop/commit/223f9bfdce23467735057bef3f585c6292cd450a))
- **menu:** reliably dispatch Debug → Style Guide across platforms\n\n- Track actual MenuId via MenuIds state and compare in on_menu_event\n- Use id 'debug_styleguide' (no dots); store resolved id string\n- Log menu ids and handle action even if platform remaps ids ([81a9bff](https://github.com/fenrick/aideon-desktop/commit/81a9bffcd4fd653b541bd28b9677ef671f218625))
- **menus:** add better menu options ([f1c0b65](https://github.com/fenrick/aideon-desktop/commit/f1c0b65a59bf54108545764732c3aeb0f77f8f2a))
- **renderer:** ensure tauri bridge before stateAt; log menu events\n\n- +page.svelte: import tauri-shim on demand if aideon missing\n- Add root platform class for consistent token behavior\n- Rust: log menu events and specific actions; log in open_styleguide() ([5345b91](https://github.com/fenrick/aideon-desktop/commit/5345b91f8c803defbb566e517c5dbd538a9fda4b))
- **splash:** fix splash page ([931ffff](https://github.com/fenrick/aideon-desktop/commit/931ffff63c06d45fe38675b1081ae6f50d9beb35))
- **svelte:** routing ([f4b2938](https://github.com/fenrick/aideon-desktop/commit/f4b293895479b5e0ef4bd91bf5ee8460368f8d50))
- **tauri:** remove invalid security.capabilities from v2 config; simplify host builder (Refs [#95](https://github.com/fenrick/aideon-desktop/issues/95)) ([691442d](https://github.com/fenrick/aideon-desktop/commit/691442da0e5445fac065c59fbb2b13c627da8688))
- **theme:** platform toggle now drives --color-accent with distinct demo colors; convert internal state to ; reduce Svelte warnings\n\n- Style: map mac/win/linux to visible accent colors for demos\n- Refs: use in styleguide and inputs to silence non-reactive warnings\n- A11y: Tabs close control is keyboard-operable\n- Cleanup: remove unused splash selector\n\nNote: slot deprecation warnings remain; planning snippet migration separately. ([cd1c99e](https://github.com/fenrick/aideon-desktop/commit/cd1c99e07890c62dc6793086a7930ab8230ae5e6))
- **worker/cli:** satisfy pyright by narrowing json result before dict.get() ([6602173](https://github.com/fenrick/aideon-desktop/commit/660217346c8f318c6002f5c93835ac96932a27dc))
- **worker/cli:** silence IPC notifications ([2c7648f](https://github.com/fenrick/aideon-desktop/commit/2c7648f343a5c146e11ad2afc55a95a06fcb9572))
- **worker/lint:** import AsyncIterator from collections.abc (Refs [#95](https://github.com/fenrick/aideon-desktop/issues/95)) ([cd80dc4](https://github.com/fenrick/aideon-desktop/commit/cd80dc4e078d984e9790fc9956a29cc8cf480f02))
- **worker:** type-safe IPC and legacy param parsing; satisfy mypy and ruff (use collections.abc.Mapping) ([fc90c74](https://github.com/fenrick/aideon-desktop/commit/fc90c74121f61fd4e70ae32b9f07839cbeccca4c))

### Features

- **app:** make renderer mount coverage-safe (dynamic import + noop in tests); add vitest config for Svelte transform ([1571fd7](https://github.com/fenrick/aideon-desktop/commit/1571fd70bc0a7df272b69c42e90830a75894214d))
- **app:** Desktop mode uses in-process engines; no inter-process RPC. ([#25](https://github.com/fenrick/aideon-desktop/issues/25), [#62](https://github.com/fenrick/aideon-desktop/issues/62)) ([0a90f7b](https://github.com/fenrick/aideon-desktop/commit/0a90f7b113a4644922526830b05a9a6c8089f2ac))
- **debug:** add Style Guide window and Debug menu item\n\n- Tauri menu: add Debug > UI Style Guide\n- Windows: new open_styleguide() opens /styleguide route\n- App: platform toggle in /styleguide to preview mac/win/linux token effects\n- CSS: fix platform class scoping (:root.platform-\*) ([f9ca5ca](https://github.com/fenrick/aideon-desktop/commit/f9ca5cad44ca9b811a5e555672c1194651a325d4))
- **design-system M0:** tokens v1, core components, layout, styleguide\n\n- Tokens (roles/elevation/spacing) and themed root\n- Core components: Button, Field, TextField, Select, Checkbox, Radio, Switch\n- Primitives: Toolbar(+Button), Tooltip, Toast host, SplitPane\n- Adaptive registries already added (shapes/docs/editors) with tests\n- Styleguide route (/styleguide) to preview tokens and components\n- Refactor Titlebar/MainView to tokens; lint/typecheck/test green ([64b903e](https://github.com/fenrick/aideon-desktop/commit/64b903e6506613d9aa668f5e4de453d7a454491a))
- **desktop:** adopt sveltekit renderer with splash route ([f07c742](https://github.com/fenrick/aideon-desktop/commit/f07c742efeceba25f40cf38463eae75e8659b33e))
- **docs): add C4 DSL and CI rendering\n\n- Add Structurizr DSL under docs/c4 with System Context + Container\n- Add scripts/render-c4.sh for local export (Structurizr CLI + PlantUML)\n- Add GitHub Action job to export PlantUML and render PNG, upload as artifacts\n\ndocs(adr): accept RPC ADR and add adapter boundaries\n\n- Mark ADR-0002 (RPC protocol) as Accepted (2025-10-28)\n- Add ADR-0003 documenting Graph/Storage/Worker adapter boundaries\n\nchore(ci): fix CODEOWNERS for monorepo layout\n\n- Map /app, /crates, /docs and add catch-all\n\nchore(tauri:** standardize build command and minor formatting\n\n- Use pnpm filter for beforeBuildCommand\n- Minor formatting in windows.rs ([bdd1bf4](https://github.com/fenrick/aideon-desktop/commit/bdd1bf49f7cdbce53a1cd1edec0582b42c692801))
- **host:** add feature-detected Tauri invoke wrapper + unit test (Refs [#102](https://github.com/fenrick/aideon-desktop/issues/102), [#95](https://github.com/fenrick/aideon-desktop/issues/95)) ([a7f3337](https://github.com/fenrick/aideon-desktop/commit/a7f333703ed33ae2d45984f92572aff3d2d6d75c))
- **host:** bootstrap minimal Tauri app + renderer shim; add dev/build scripts (Refs [#96](https://github.com/fenrick/aideon-desktop/issues/96), [#95](https://github.com/fenrick/aideon-desktop/issues/95)) ([68c89e2](https://github.com/fenrick/aideon-desktop/commit/68c89e29ddb671e03f3d2572aef503df3f5154b4))
- **host:** embed rust temporal engine and workspace ([d47a314](https://github.com/fenrick/aideon-desktop/commit/d47a3140792a5a86a3a6b4eb2f90f53fd4f36330))
- **host:** modularize tauri host ([78b3a3b](https://github.com/fenrick/aideon-desktop/commit/78b3a3b0d59d7f8a5d8044fc4761e6664870afd9))
- **logging:** broaden renderer and host observability ([5bc79a2](https://github.com/fenrick/aideon-desktop/commit/5bc79a2c82e5fd9365fe53a277646d8a2754165c))
- **logging:** enrich telemetry and stabilize type tooling ([18b549b](https://github.com/fenrick/aideon-desktop/commit/18b549b4ab2446d8014c6d3a09e1315755dcb25b))
- **m1:** Praxis canvas foundations + Sonar/coverage hardening ([#137](https://github.com/fenrick/aideon-desktop/issues/137)) ([082ce46](https://github.com/fenrick/aideon-desktop/commit/082ce467fd66540b2886816dfacb918979825b3f))
- **rpc:** selectively restore IPC changes from 12209b6 (exclude generated .aideon and docs/issues) ([4a79840](https://github.com/fenrick/aideon-desktop/commit/4a79840e38fad83688cf5ff2f0f99b73d85241d7))
- **ui:** introduce adaptive layer primitives\n\n- Tokens: add CSS design tokens and wire to theme\n- UI primitives: Tabs (multi-document), Modal (accessible skeleton)\n- Registries: shapes, documents, property editors (extensible) with tests\n\nstyle: normalize naming for lint rules; tests cover registries ([0192e5c](https://github.com/fenrick/aideon-desktop/commit/0192e5c8e64080276475e6a34e904b12cb7a2f41))
- **ui:** platform-aware design system parity (Windows/macOS/Neutral)\n\n- Add theme manager and OS preview; inject Fluent/Puppertino/Tailwind on demand\n- Wrap Button, TextField, Select, Checkbox, Radio, Switch, ToolbarButton, Tooltip, Modal\n- Style Guide: platform toggle reactive; remove unused selectors; fix a11y/events\n- Tests: ensure Tailwind only loads for Neutral; platform class toggles\n\nRefs: M0 foundations, prepares M1 MVP UI ([449f3d6](https://github.com/fenrick/aideon-desktop/commit/449f3d63de83eb33fab71b4ee1853c537408a89f))
- **worker): add /health endpoint; feat(app): use health check for readiness; ci(sonar:** wait for Quality Gate; docs: add 80% coverage gates to AGENTS.md and reference branch for new code ([de5da2b](https://github.com/fenrick/aideon-desktop/commit/de5da2b9bd5ff1ed55ad011e952b810d8163b40d))
- **worker:** add server and remove CLI; Desktop mode uses in-process engines; no inter-process RPC. (Refs [#95](https://github.com/fenrick/aideon-desktop/issues/95)) ([2329521](https://github.com/fenrick/aideon-desktop/commit/2329521cb84819eab990bc466e8e3bfd002abd56))

# [0.1.0](https://github.com/fenrick/aideon-praxis/compare/v0.0.0...v0.1.0) (2025-10-18)

### Bug Fixes

- **app:** rename catch binding; export AideonApi from global types to avoid bare export ([b981a7a](https://github.com/fenrick/aideon-praxis/commit/b981a7a782382e847e24eb5f187b621929a335d0))
- **app:** resolve ESLint issues (prefer globalThis, typed bridge, event-based ready, remove unnecessary conditions) ([0598ea2](https://github.com/fenrick/aideon-praxis/commit/0598ea21755d9cfc912729e9e2acc8a5cd395b3e))
- **app:** solidify global typings with module augmentation; use typed bridge access; update tests for require-await ([31b2de1](https://github.com/fenrick/aideon-praxis/commit/31b2de17e93c38d727104e74c5fc66ff538b407f))
- **dev:** wait for renderer index.html before launching Electron; add wait-on to coordinate dev watch builds ([b61392d](https://github.com/fenrick/aideon-praxis/commit/b61392db961421f8a17740dddc0fe77d8cacc249))
- **ipc:** register worker IPC handlers before readiness; await READY internally to avoid 'No handler registered' race ([51c1709](https://github.com/fenrick/aideon-praxis/commit/51c1709a752dc1d5e5411bad3fbe291708493748))
- **lint:** reduce main() complexity, remove fs loop, use helper for spawn; rename variables to satisfy rules; add vite/client types ([239ea8b](https://github.com/fenrick/aideon-praxis/commit/239ea8b866e729c8afa644ed5372a7ef3c8bc615))
- **renderer:** set Vite base='./' so built assets load via file:// in Electron ([2ae8828](https://github.com/fenrick/aideon-praxis/commit/2ae882836eb515327b8ce5fa787add86db98be17))

### Features

- **adapters:** add adapter interfaces and unit test ([2a4a814](https://github.com/fenrick/aideon-praxis/commit/2a4a8146bce8357fba130f2b5c980f6edafb2e78))
- **app:** expose typed stateAt; Renderer calls Host via Tauri invoke (typed adapters). ([ebc9948](https://github.com/fenrick/aideon-praxis/commit/ebc994838d6674c1b753eead7204142091890efe))
- **app:** scaffold secure Electron host + React renderer with packaging ([fec07a4](https://github.com/fenrick/aideon-praxis/commit/fec07a442af9c4888b8090e71eaf48c2e7687410))
- **e2e:** wire renderer↔host↔engines roundtrip; Renderer calls Host via Tauri invoke (typed adapters). Desktop mode runs engines in-process. Host calls engines via Rust traits. No sockets. ([1ce514e](https://github.com/fenrick/aideon-praxis/commit/1ce514e19cfd183a67966f862ca2bbef337b0c94))
- **worker:** add Python sidecar with Temporal.StateAt and CLI ([1facad5](https://github.com/fenrick/aideon-praxis/commit/1facad5388ad795257fa65a21340b39f60288074))
