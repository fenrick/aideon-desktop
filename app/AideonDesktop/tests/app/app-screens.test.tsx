import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  AboutScreen,
  FrontendReady,
  MainScreen,
  SettingsScreen,
  SplashScreenRoute,
  StatusScreen,
  StyleguideScreen,
} from '@/app/app-screens';
import { ColorThemeProvider } from 'design-system/theme/color-theme';
import { isTauriRuntime } from 'lib/runtime';

vi.mock('@/root', () => ({ AideonDesktopRoot: () => <div>Root</div> }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn().mockResolvedValue(true) }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ label: 'main' }),
}));

afterEach(() => {
  cleanup();
  delete (globalThis as { __TAURI__?: unknown }).__TAURI__;
  delete (globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  vi.clearAllMocks();
  vi.useRealTimers();
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

  it('cycles splash screen copy', () => {
    vi.useFakeTimers();
    render(<SplashScreenRoute />);

    expect(screen.getByText('Reticulating splines…')).toBeInTheDocument();
    act(() => {
      vi.advanceTimersByTime(1600);
    });
    expect(screen.getByText('Weaving twin orbits…')).toBeInTheDocument();
  });

  it('renders status and about screens', () => {
    render(<StatusScreen />);
    expect(screen.getByText('Host status')).toBeInTheDocument();
    expect(screen.getByText('All services initialising…')).toBeInTheDocument();

    render(<AboutScreen />);
    expect(screen.getByText('Aideon')).toBeInTheDocument();
    expect(screen.getByText('Desktop shell for Praxis workspace and tools.')).toBeInTheDocument();
  });

  it('renders settings and styleguide screens', () => {
    render(
      <ColorThemeProvider>
        <SettingsScreen />
      </ColorThemeProvider>,
    );

    expect(screen.getByText('Settings')).toBeInTheDocument();
    expect(screen.getByText('Color theme')).toBeInTheDocument();
    expect(screen.getByText('Corporate Blue')).toBeInTheDocument();
    expect(screen.getByText('Default')).toBeInTheDocument();

    render(<StyleguideScreen />);
    expect(screen.getByText('Styleguide')).toBeInTheDocument();
    expect(screen.getByText('Design system documentation pending.')).toBeInTheDocument();
  });
});
