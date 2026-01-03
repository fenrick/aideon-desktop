import { render, screen } from '@testing-library/react';
import type { SelectionState } from 'aideon/canvas/types';
import { describe, expect, it, vi } from 'vitest';

vi.mock('praxis/components/chrome/praxis-workspace-toolbar', () => ({
  PraxisWorkspaceToolbar: () => <div>Praxis Workspace Toolbar</div>,
}));

vi.mock('praxis/components/template-screen/overview-tabs', () => ({
  OverviewTabs: () => <div>Overview Tabs</div>,
}));

vi.mock('praxis/components/template-screen/projects-sidebar', () => ({
  ProjectsSidebar: () => <div>Projects Sidebar</div>,
}));

vi.mock('praxis/components/template-screen/properties-inspector', () => ({
  PropertiesInspector: ({ selection }: { selection: SelectionState }) => (
    <div>Properties Inspector ({selection.nodeIds.length + selection.edgeIds.length})</div>
  ),
}));

vi.mock('praxis/domain-data', () => ({
  listProjectsWithScenarios: vi.fn().mockResolvedValue([
    {
      id: 'p1',
      name: 'Project One',
      scenarios: [
        {
          id: 'main',
          name: 'Main Scenario',
          branch: 'main',
          updatedAt: '2025-01-01T00:00:00Z',
          isDefault: true,
        },
      ],
    },
  ]),
  listTemplatesFromHost: vi
    .fn()
    .mockResolvedValue([
      {
        id: 'template-1',
        documentId: 'canvasdoc-1',
        name: 'Template 1',
        description: 'desc',
        widgets: [],
      },
    ]),
}));

vi.mock('praxis/praxis-api', () => ({
  listScenarios: vi.fn().mockResolvedValue([
    {
      id: 'main',
      name: 'Main Scenario',
      branch: 'main',
      updatedAt: '2025-01-01T00:00:00Z',
      isDefault: true,
    },
  ]),
  applyOperations: vi.fn(),
}));

vi.mock('praxis/platform', () => ({ isTauri: vi.fn(() => false) }));

import { PraxisWorkspaceSurface } from 'praxis/workspace';

describe('PraxisWorkspaceSurface', () => {
  it('renders the new shell layout pieces', async () => {
    render(<PraxisWorkspaceSurface />);

    expect(await screen.findByText(/Projects Sidebar/)).toBeInTheDocument();
    expect(screen.getByText(/Praxis Workspace Toolbar/)).toBeInTheDocument();
    expect(screen.getByText(/Overview Tabs/)).toBeInTheDocument();
    expect(screen.getByText(/Properties Inspector/)).toBeInTheDocument();
  });
});
