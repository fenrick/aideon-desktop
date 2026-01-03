import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { SelectionState } from 'aideon/canvas/types';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('praxis/components/template-screen/projects-sidebar', () => ({
  ProjectsSidebar: ({ projects, scenarios }: { projects: unknown[]; scenarios: unknown[] }) => (
    <div data-testid="projects-sidebar">
      {projects.length > 0 ? projects.length : scenarios.length} scenarios
    </div>
  ),
}));

vi.mock('praxis/components/chrome/praxis-workspace-toolbar', () => ({
  PraxisWorkspaceToolbar: ({
    templates,
    activeTemplateId,
    onTemplateSave,
    onTemplateChange,
    loading,
  }: {
    templates: { id: string }[];
    activeTemplateId: string;
    onTemplateSave: () => void;
    onTemplateChange: (id: string) => void;
    loading?: boolean;
  }) => (
    <div data-testid="praxis-toolbar">
      <span data-testid="active-template">{activeTemplateId}</span>
      <span data-testid="template-count">{templates.length}</span>
      {loading ? <span>loading</span> : undefined}
      <button onClick={onTemplateSave}>Save template</button>
      <button
        onClick={() => {
          onTemplateChange('alt-template');
        }}
      >
        Change template
      </button>
    </div>
  ),
}));

vi.mock('praxis/components/template-screen/overview-tabs', () => ({
  OverviewTabs: ({
    onSelectionChange,
    reloadSignal,
  }: {
    onSelectionChange: (selection: {
      nodeIds: string[];
      edgeIds: string[];
      sourceWidgetId?: string;
    }) => void;
    reloadSignal?: number;
  }) => (
    <div>
      <button
        onClick={() => {
          onSelectionChange({ nodeIds: ['n1'], edgeIds: [], sourceWidgetId: 'widget-1' });
        }}
      >
        simulate-selection
      </button>
      <span data-testid="reload-signal">{reloadSignal ?? 0}</span>
    </div>
  ),
}));

vi.mock('praxis/components/template-screen/properties-inspector', () => ({
  PropertiesInspector: ({
    selectionKind,
    selectionId,
    selection,
    saving,
  }: {
    selectionKind: string;
    selectionId?: string;
    selection: SelectionState;
    saving?: boolean;
  }) => (
    <div data-testid="inspector">
      {selectionKind}:{selectionId}:{selection.nodeIds.length + selection.edgeIds.length}:
      {saving ? 'saving' : 'idle'}
    </div>
  ),
}));

vi.mock('praxis/domain-data', () => ({
  listProjectsWithScenarios: vi.fn(),
  listTemplatesFromHost: vi.fn(),
}));

vi.mock('praxis/praxis-api', () => ({
  listScenarios: vi.fn(),
  applyOperations: vi.fn(),
}));

vi.mock('praxis/platform', () => ({ isTauri: vi.fn() }));

import { listProjectsWithScenarios, listTemplatesFromHost } from 'praxis/domain-data';
import { searchStore } from 'praxis/lib/search';
import { isTauri } from 'praxis/platform';
import { listScenarios } from 'praxis/praxis-api';
import { PraxisWorkspaceSurface } from 'praxis/workspace';

const listScenariosMock = vi.mocked(listScenarios);
const listProjectsMock = vi.mocked(listProjectsWithScenarios);
const listTemplatesMock = vi.mocked(listTemplatesFromHost);
const isTauriMock = vi.mocked(isTauri);

describe('Praxis canvas app shell', () => {
  beforeEach(() => {
    listScenariosMock.mockResolvedValue([
      {
        id: 'mainline',
        name: 'Mainline FY25',
        branch: 'main',
        updatedAt: '2025-01-01T00:00:00Z',
        isDefault: true,
      },
    ]);
    listProjectsMock.mockResolvedValue([
      {
        id: 'project-1',
        name: 'Project 1',
        scenarios: [
          {
            id: 'mainline',
            name: 'Mainline FY25',
            branch: 'main',
            updatedAt: '2025-01-01T00:00:00Z',
            isDefault: true,
          },
        ],
      },
    ]);
    listTemplatesMock.mockResolvedValue([
      {
        id: 'template-a',
        documentId: 'canvasdoc-a',
        name: 'Template A',
        description: 'desc',
        widgets: [
          {
            id: 'widget-1',
            kind: 'graph',
            title: 'Graph',
            size: 'full',
            view: {
              id: 'graph-default',
              name: 'Graph',
              kind: 'graph',
              filters: {},
            },
          },
        ],
      },
    ]);
    isTauriMock.mockReturnValue(false);
    searchStore.clear();
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it('renders the shell, fetches scenarios, and shows navigation + header', async () => {
    render(<PraxisWorkspaceSurface />);

    await waitFor(() => {
      expect(listProjectsMock).toHaveBeenCalled();
      expect(listTemplatesMock).toHaveBeenCalled();
    });
    expect(screen.getByTestId('projects-sidebar')).toHaveTextContent('1 scenarios');
    expect(screen.getByTestId('active-template')).toBeInTheDocument();
    expect(screen.getByTestId('praxis-toolbar')).toBeInTheDocument();
  });

  it('forwards selection updates to parent callback', async () => {
    const onSelectionChange = vi.fn();

    render(<PraxisWorkspaceSurface onSelectionChange={onSelectionChange} />);

    fireEvent.click(screen.getByText('simulate-selection'));

    await waitFor(() => {
      expect(onSelectionChange.mock.calls.length).toBeGreaterThanOrEqual(1);
    });
  });

  it('saves a template via header action', async () => {
    render(<PraxisWorkspaceSurface />);

    const initialCount = Number(screen.getByTestId('template-count').textContent);
    fireEvent.click(screen.getByText('Save template'));

    await waitFor(() => {
      const updated = Number(screen.getByTestId('template-count').textContent);
      expect(updated).toBeGreaterThan(initialCount);
    });
  });
});
