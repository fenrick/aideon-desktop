import { afterEach, describe, expect, it } from 'vitest';

import { isTauri } from '@/workspaces/mneme/platform';

const tauriWindow = globalThis as unknown as Window & {
  __TAURI_INTERNALS__?: unknown;
  __TAURI_METADATA__?: unknown;
};

afterEach(() => {
  delete tauriWindow.__TAURI_INTERNALS__;
  delete tauriWindow.__TAURI_METADATA__;
});

describe('mneme platform', () => {
  it('returns false when no tauri flags are present', () => {
    expect(isTauri()).toBe(false);
  });

  it('detects tauri internals and metadata flags', () => {
    tauriWindow.__TAURI_INTERNALS__ = {};
    expect(isTauri()).toBe(true);

    delete tauriWindow.__TAURI_INTERNALS__;
    tauriWindow.__TAURI_METADATA__ = {};
    expect(isTauri()).toBe(true);
  });
});
