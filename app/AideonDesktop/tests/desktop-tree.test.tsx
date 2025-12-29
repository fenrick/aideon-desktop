import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { SidebarProvider } from './design-system';

vi.mock('praxis', () => ({
  listScenarios: vi
    .fn()
    .mockResolvedValue([
      { id: 'p1-w1', name: 'Workspace · Default', branch: 'main', updatedAt: '2025-11-01T00:00Z' },
    ]),
}));

import { listScenarios } from 'praxis';

import { DesktopTree } from './desktop-tree';

describe('DesktopTree', () => {
  it('renders stub projects and workspaces', () => {
    render(
      <SidebarProvider>
        <DesktopTree />
      </SidebarProvider>,
    );

    return waitFor(() => {
      expect(screen.getByText('Scenarios')).toBeInTheDocument();
      expect(screen.getByText('Workspace · Default')).toBeInTheDocument();
    });
  });

  it('shows an error when loading fails', async () => {
    vi.mocked(listScenarios).mockRejectedValueOnce(new Error('offline'));

    render(
      <SidebarProvider>
        <DesktopTree />
      </SidebarProvider>,
    );

    await waitFor(() => {
      expect(screen.getByText(/Failed to load workspaces/i)).toBeInTheDocument();
    });
  });
});
