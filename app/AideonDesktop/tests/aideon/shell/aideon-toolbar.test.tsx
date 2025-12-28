import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { AideonToolbar } from 'aideon/shell/aideon-toolbar';
import { AideonShellControlsProvider } from 'aideon/shell/shell-controls';

const toggleSidebar = vi.fn();

vi.mock('design-system/desktop-shell', async () => {
  const menubar = await import('design-system/components/ui/menubar');
  return {
    Menubar: menubar.Menubar,
    MenubarContent: menubar.MenubarContent,
    MenubarItem: menubar.MenubarItem,
    MenubarMenu: menubar.MenubarMenu,
    MenubarTrigger: menubar.MenubarTrigger,
    SidebarTrigger: ({ className }: { readonly className?: string }) => (
      <button
        type="button"
        className={className}
        onClick={() => {
          toggleSidebar();
        }}
      />
    ),
    useSidebar: () => ({ toggleSidebar }),
  };
});

afterEach(() => {
  cleanup();
  toggleSidebar.mockClear();
});

describe('AideonToolbar', () => {
  it('handles browser shortcuts and opens the command palette', () => {
    const toggleInspector = vi.fn();

    render(
      <AideonShellControlsProvider value={{ inspectorCollapsed: false, toggleInspector }}>
        <AideonToolbar title="Aideon" modeLabel="Browser preview" />
      </AideonShellControlsProvider>,
    );

    fireEvent.keyDown(globalThis, { key: 'b', ctrlKey: true });
    expect(toggleSidebar).toHaveBeenCalledTimes(1);

    fireEvent.keyDown(globalThis, { key: 'i', ctrlKey: true });
    expect(toggleInspector).toHaveBeenCalledTimes(1);

    fireEvent.keyDown(globalThis, { key: 'k', ctrlKey: true });
    expect(screen.getByPlaceholderText('Search commands…')).toBeInTheDocument();
    expect(screen.getByText('Toggle navigation')).toBeInTheDocument();
    expect(screen.getByText('Toggle inspector')).toBeInTheDocument();
  });

  it('does not trigger shortcuts while typing in an input', () => {
    const toggleInspector = vi.fn();

    render(
      <AideonShellControlsProvider value={{ inspectorCollapsed: false, toggleInspector }}>
        <AideonToolbar
          title="Aideon"
          modeLabel="Browser preview"
          center={<input aria-label="Search" />}
        />
      </AideonShellControlsProvider>,
    );

    const input = screen.getByLabelText('Search');
    fireEvent.keyDown(input, { key: 'k', ctrlKey: true });

    expect(screen.queryByPlaceholderText('Search commands…')).not.toBeInTheDocument();
  });

  it('opens keyboard shortcuts from the command palette', () => {
    const toggleInspector = vi.fn();

    render(
      <AideonShellControlsProvider value={{ inspectorCollapsed: false, toggleInspector }}>
        <AideonToolbar title="Aideon" modeLabel="Browser preview" />
      </AideonShellControlsProvider>,
    );

    fireEvent.keyDown(globalThis, { key: 'k', ctrlKey: true });
    fireEvent.click(screen.getByText('Keyboard shortcuts…'));

    expect(screen.getByText('Keyboard shortcuts')).toBeInTheDocument();
  });
});
