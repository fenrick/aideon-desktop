import type { ReactNode } from 'react';

import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('react-resizable-panels', () => ({
  PanelGroup: ({ children }: { readonly children: ReactNode }) => <div>{children}</div>,
  Panel: ({ children }: { readonly children: ReactNode }) => <div>{children}</div>,
  PanelResizeHandle: () => <div aria-label="Resize handle" />,
}));

import {
  Menubar,
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
  Sidebar,
  SidebarProvider,
} from 'design-system/desktop-shell';
import { DesktopShell } from 'design-system/desktop-shell/desktop-shell';

describe('desktop-shell proxies', () => {
  it('exposes shadcn primitives through the design system', () => {
    expect(Sidebar).toBeDefined();
    expect(SidebarProvider).toBeDefined();
    expect(ResizablePanelGroup).toBeDefined();
    expect(ResizablePanel).toBeDefined();
    expect(ResizableHandle).toBeDefined();
    expect(Menubar).toBeDefined();
  });
});

describe('DesktopShell', () => {
  it('renders the provided slots', () => {
    render(
      <DesktopShell
        toolbar={<div>Toolbar</div>}
        tree={
          <Sidebar collapsible="icon" className="h-full">
            <div>Tree</div>
          </Sidebar>
        }
        main={<div>Main</div>}
        properties={<div>Properties</div>}
      />,
    );

    expect(screen.getByText('Toolbar')).toBeInTheDocument();
    expect(screen.getByText('Tree')).toBeInTheDocument();
    expect(screen.getByText('Main')).toBeInTheDocument();
    expect(screen.getByText('Properties')).toBeInTheDocument();
  });
});
