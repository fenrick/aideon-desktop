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

vi.mock('./workspaces/registry', () => {
  const workspace = {
    id: 'praxis',
    label: 'Praxis',
    Host: ({
      children,
      workspaceSwitcher,
    }: {
      children: (slots: {
        toolbar: ReactNode;
        navigation: ReactNode;
        content: ReactNode;
        inspector: ReactNode;
        overlays?: ReactNode;
      }) => ReactNode;
      workspaceSwitcher: { currentId: string };
    }) =>
      children({
        toolbar: <div>Toolbar {workspaceSwitcher.currentId}</div>,
        navigation: <div>Navigation</div>,
        content: <div>Content</div>,
        inspector: <div>Inspector</div>,
        overlays: <div>Overlay</div>,
      }),
  };

  return {
    listWorkspaceOptions: () => [{ id: 'praxis', label: 'Praxis', disabled: false }],
    resolveWorkspace: () => workspace,
  };
});

import { AideonDesktopRoot } from './root';

describe('AideonDesktopRoot', () => {
  it('renders the active workspace slots through the shell', () => {
    render(<AideonDesktopRoot />);

    expect(screen.getByText('Toolbar praxis')).toBeInTheDocument();
    expect(screen.getByText('Navigation')).toBeInTheDocument();
    expect(screen.getByText('Content')).toBeInTheDocument();
    expect(screen.getByText('Inspector')).toBeInTheDocument();
    expect(screen.getByText('Overlay')).toBeInTheDocument();
  });
});
