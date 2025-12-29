import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { FrontendReady, MainScreen } from '@/app/app-screens';
import { isTauriRuntime } from 'lib/runtime';

vi.mock('@/root', () => ({ AideonDesktopRoot: () => <div>Root</div> }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn().mockResolvedValue(true) }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ label: 'main' }),
}));

afterEach(() => {
  delete (globalThis as { __TAURI__?: unknown }).__TAURI__;
  delete (globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  vi.clearAllMocks();
});

describe('app screens', () => {
  it('renders the main screen in browser mode', () => {
    render(<MainScreen />);
    expect(screen.getByText('Root')).toBeInTheDocument();
  });

  it('signals frontend readiness in tauri', async () => {
    (globalThis as { __TAURI__?: unknown }).__TAURI__ = {};
    const { invoke } = await import('@tauri-apps/api/core');

    render(
      <FrontendReady>
        <div>ready</div>
      </FrontendReady>,
    );

    expect(screen.getByText('ready')).toBeInTheDocument();
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('set_complete', { task: 'frontend' });
    });
  });

  it('detects tauri runtime from global flags', () => {
    expect(isTauriRuntime()).toBe(false);
    (globalThis as { __TAURI__?: unknown }).__TAURI__ = {};
    expect(isTauriRuntime()).toBe(true);
  });
});
