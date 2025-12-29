import { render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('aideon/shell/aideon-desktop-shell', () => ({
  AideonDesktopShell: ({
    toolbar,
    navigation,
    content,
    inspector,
  }: {
    toolbar: ReactNode;
    navigation: ReactNode;
    content: ReactNode;
    inspector: ReactNode;
  }) => (
    <div>
      <div>{toolbar}</div>
      <div>{navigation}</div>
      <div>{content}</div>
      <div>{inspector}</div>
    </div>
  ),
}));

vi.mock('./workspaces/praxis/workspace', () => ({
  PraxisWorkspaceProvider: ({ children }: { children: ReactNode }) => <>{children}</>,
}));

vi.mock('./workspaces/registry', () => {
  const workspace = {
    id: 'praxis',
    label: 'Praxis',
    enabled: true,
    Navigation: () => <div>Navigation</div>,
    Toolbar: () => <div>Toolbar</div>,
    Content: () => <div>Content</div>,
    Inspector: () => <div>Inspector</div>,
  };

  return {
    getWorkspaceOptions: () => [{ id: 'praxis', label: 'Praxis', disabled: false }],
    getWorkspace: () => workspace,
  };
});

import { AideonDesktopRoot } from './root';

describe('AideonDesktopRoot', () => {
  it('renders the active workspace slots through the shell', () => {
    render(<AideonDesktopRoot />);

    expect(screen.getByText('Toolbar')).toBeInTheDocument();
    expect(screen.getByText('Navigation')).toBeInTheDocument();
    expect(screen.getByText('Content')).toBeInTheDocument();
    expect(screen.getByText('Inspector')).toBeInTheDocument();
  });
});
