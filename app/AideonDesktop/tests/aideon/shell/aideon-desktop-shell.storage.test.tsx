import { fireEvent, render, screen } from '@testing-library/react';
import type { PropsWithChildren } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const onLayoutCalls: number[][] = [];

vi.mock('design-system/components/ui/scroll-area', () => ({
  ScrollArea: ({ children, ...properties }: PropsWithChildren<Record<string, unknown>>) => (
    <div {...properties}>{children}</div>
  ),
}));

vi.mock('design-system/desktop-shell', () => ({
  SidebarProvider: ({ children }: PropsWithChildren) => <div>{children}</div>,
  Sidebar: ({ children }: PropsWithChildren) => <div>{children}</div>,
  SidebarInset: ({ children }: PropsWithChildren) => <div>{children}</div>,
  ResizableHandle: () => <div />,
  ResizablePanel: ({ children }: PropsWithChildren) => <div>{children}</div>,
  ResizablePanelGroup: ({
    children,
    onLayout,
  }: PropsWithChildren<{ onLayout?: (sizes: number[]) => void }>) => {
    if (onLayout) {
      const sizes = [18, 62, 20];
      onLayoutCalls.push(sizes);
      onLayout(sizes);
    }
    return <div>{children}</div>;
  },
}));

vi.mock('design-system/lib/utilities', () => ({
  cn: (...classes: (string | undefined)[]) => classes.filter(Boolean).join(' '),
}));

import { AideonDesktopShell } from 'aideon/shell/aideon-desktop-shell';
import { useAideonShellControls } from 'aideon/shell/shell-controls';

/**
 * Patch `globalThis.localStorage` for local layout tests.
 * @param storage replacement storage implementation
 */
function setLocalStorage(storage: Storage | undefined) {
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: storage,
  });
}

describe('AideonDesktopShell storage', () => {
  beforeEach(() => {
    onLayoutCalls.length = 0;
  });

  it('renders without toolbar and ignores missing localStorage', () => {
    setLocalStorage(undefined);

    render(
      <AideonDesktopShell
        navigation={<div>Nav</div>}
        content={<div>Content</div>}
        inspector={<div>Inspector</div>}
      />,
    );

    expect(screen.getByText('Nav')).toBeInTheDocument();
    expect(screen.getByText('Content')).toBeInTheDocument();
    expect(screen.getByText('Inspector')).toBeInTheDocument();
    expect(screen.queryByText('Toolbar')).not.toBeInTheDocument();
  });

  it('reads stored layout when valid and tolerates invalid JSON', () => {
    const getItem = vi
      .fn()
      .mockReturnValueOnce('not-json')
      .mockReturnValueOnce(JSON.stringify([12, 70, 18]));
    const setItem = vi.fn();
    setLocalStorage({ getItem, setItem } as unknown as Storage);

    const { unmount } = render(
      <AideonDesktopShell
        navigation={<div>Nav</div>}
        content={<div>Content</div>}
        inspector={<div>Inspector</div>}
        toolbar={<div>Toolbar</div>}
      />,
    );
    expect(screen.getByText('Toolbar')).toBeInTheDocument();
    unmount();

    render(
      <AideonDesktopShell
        navigation={<div>Nav</div>}
        content={<div>Content</div>}
        inspector={<div>Inspector</div>}
      />,
    );
    expect(getItem).toHaveBeenCalledWith('aideon-shell-panels');
  });

  it('persists inspector collapse state when toggled', () => {
    const getItem = vi.fn();
    const setItem = vi.fn();
    setLocalStorage({ getItem, setItem } as unknown as Storage);

    /**
     *
     */
    function ToggleInspector() {
      const shell = useAideonShellControls();
      return (
        <button
          type="button"
          onClick={() => {
            shell?.toggleInspector();
          }}
        >
          toggle
        </button>
      );
    }

    render(
      <AideonDesktopShell
        navigation={<div>Nav</div>}
        content={<div>Content</div>}
        inspector={<div>Inspector</div>}
        toolbar={<ToggleInspector />}
      />,
    );

    fireEvent.click(screen.getByText('toggle'));
    expect(setItem).toHaveBeenCalledWith('aideon-shell-inspector-collapsed', '1');
  });

  it('writes layout sizes when storage allows, and ignores write failures', () => {
    const getItem = vi.fn().mockReturnValue(JSON.stringify([20, 60, 20]));
    const setItem = vi.fn();
    setLocalStorage({ getItem, setItem } as unknown as Storage);

    render(
      <AideonDesktopShell
        navigation={<div>Nav</div>}
        content={<div>Content</div>}
        inspector={<div>Inspector</div>}
      />,
    );

    expect(onLayoutCalls.length).toBeGreaterThan(0);
    expect(setItem).toHaveBeenCalledWith('aideon-shell-panels', JSON.stringify([18, 62, 20]));

    setItem.mockImplementationOnce(() => {
      throw new Error('quota');
    });

    render(
      <AideonDesktopShell
        navigation={<div>Nav</div>}
        content={<div>Content</div>}
        inspector={<div>Inspector</div>}
      />,
    );

    expect(setItem).toHaveBeenCalled();
  });

  it('ignores stored layout when values are not numbers', () => {
    const storage = {
      getItem: vi.fn().mockReturnValue(JSON.stringify(['bad', 60, 20])),
      setItem: vi.fn(),
    };
    setLocalStorage(storage as unknown as Storage);

    render(
      <AideonDesktopShell
        navigation={<div>Nav</div>}
        content={<div>Content</div>}
        inspector={<div>Inspector</div>}
        toolbar={<div>Toolbar</div>}
      />,
    );

    expect(screen.getByText('Toolbar')).toBeInTheDocument();
  });
});
