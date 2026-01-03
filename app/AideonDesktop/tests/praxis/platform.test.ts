import { describe, expect, it } from 'vitest';

import { isTauri } from 'praxis/platform';

describe('praxis/platform', () => {
  it('detects tauri internals when present', () => {
    const win = globalThis.window as Window & { __TAURI_INTERNALS__?: unknown };
    expect(isTauri()).toBe(false);

    win.__TAURI_INTERNALS__ = {};
    expect(isTauri()).toBe(true);

    delete win.__TAURI_INTERNALS__;
  });
});
