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

/**
 * Narrow unknown values to plain object records.
 * @param value
 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

/**
 * Extract the requestId value from a Tauri invoke arguments object.
 * @param invokeArguments - Raw invoke args.
 */
function requestIdFromInvokeArguments(invokeArguments: unknown): string | undefined {
  if (!isRecord(invokeArguments)) {
    return undefined;
  }
  const request = invokeArguments.request;
  if (!isRecord(request)) {
    return undefined;
  }
  const requestId = request.requestId;
  return typeof requestId === 'string' ? requestId : undefined;
}

/**
 * Extract the request payload value from a Tauri invoke arguments object.
 * @param invokeArguments - Raw invoke args.
 */
function payloadFromInvokeArguments(invokeArguments: unknown): unknown {
  if (!isRecord(invokeArguments)) {
    return undefined;
  }
  const request = invokeArguments.request;
  if (!isRecord(request)) {
    return undefined;
  }
  return request.payload;
}

/**
 * Find invoke args for a given command.
 * @param calls
 * @param command
 */
function findInvokeArguments(calls: unknown[][], command: string): unknown {
  const call = calls.find((entry) => entry[0] === command);
  return call?.[1];
}

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockImplementation((_command: string, invokeArguments: unknown) => {
    const requestId = requestIdFromInvokeArguments(invokeArguments) ?? 'req';
    return Promise.resolve({ requestId, status: 'ok', result: undefined });
  }),
}));
let currentWindowLabel = 'main';
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ label: currentWindowLabel }),
}));

afterEach(() => {
  cleanup();
  delete (globalThis as { __TAURI__?: unknown }).__TAURI__;
  delete (globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  vi.clearAllMocks();
  vi.useRealTimers();
  currentWindowLabel = 'main';
});

describe('app screens', () => {
  it('renders the main screen in browser mode', () => {
    render(<MainScreen />);
    expect(screen.getByText('Root')).toBeInTheDocument();
  });

  it('signals frontend readiness in tauri', async () => {
    (globalThis as { __TAURI__?: unknown }).__TAURI__ = {};
    const { invoke } = await import('@tauri-apps/api/core');
    const invokeMock = vi.mocked(invoke);

    render(
      <FrontendReady>
        <div>ready</div>
      </FrontendReady>,
    );

    expect(screen.getByText('ready')).toBeInTheDocument();
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('system.setup.complete', expect.anything());
    });
    const calls = (invokeMock as unknown as { mock: { calls: unknown[][] } }).mock.calls;
    const invokeArguments = findInvokeArguments(calls, 'system.setup.complete');
    expect(payloadFromInvokeArguments(invokeArguments)).toEqual({ task: 'frontend' });
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

  it('signals frontend readiness from the splash window', async () => {
    (globalThis as { __TAURI__?: unknown }).__TAURI__ = {};
    currentWindowLabel = 'splash';
    const { invoke } = await import('@tauri-apps/api/core');
    const invokeMock = vi.mocked(invoke);

    render(<SplashScreenRoute />);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('system.setup.complete', expect.anything());
    });
    const calls = (invokeMock as unknown as { mock: { calls: unknown[][] } }).mock.calls;
    const invokeArguments = findInvokeArguments(calls, 'system.setup.complete');
    expect(payloadFromInvokeArguments(invokeArguments)).toEqual({ task: 'frontend' });
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
