import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { TemporalPanelActions, TemporalPanelState } from 'praxis/time/use-temporal-panel';

interface Project {
  id: string;
  name: string;
  scenarios: { id: string; name: string; branch: string; updatedAt: string; isDefault: boolean }[];
}

interface ProjectsSidebarProperties {
  projects: Project[];
  error?: string;
  onRetry?: () => void;
  onSelectScenario?: (scenarioId: string) => void;
}

vi.mock('praxis/components/template-screen/projects-sidebar', () => ({
  ProjectsSidebar: ({ projects, error, onRetry, onSelectScenario }: ProjectsSidebarProperties) => (
    <div>
      <div data-testid="projects-count">{projects.length}</div>
      {error ? <div data-testid="projects-error">{error}</div> : undefined}
      <button data-testid="retry-projects" onClick={() => onRetry?.()}>
        retry
      </button>
      <button
        data-testid="scenario-change"
        onClick={() => {
          onSelectScenario?.('alt');
        }}
      >
        scenario
      </button>
    </div>
  ),
}));
vi.mock('praxis/components/chrome/praxis-workspace-toolbar', () => ({
  PraxisWorkspaceToolbar: ({
    onTemplateSave,
    onCreateWidget,
  }: {
    onTemplateSave?: () => void;
    onCreateWidget?: () => void;
  }) => (
    <div>
      <button onClick={() => onTemplateSave?.()} data-testid="save-template">
        save
      </button>
      <button onClick={() => onCreateWidget?.()} data-testid="open-library">
        add
      </button>
    </div>
  ),
}));
vi.mock('praxis/components/template-screen/overview-tabs', () => ({
  OverviewTabs: ({
    onSelectionChange,
    reloadSignal,
    branchTriggerRef,
  }: {
    onSelectionChange: (selection: {
      nodeIds: string[];
      edgeIds: string[];
      sourceWidgetId?: string;
    }) => void;
    reloadSignal?: number;
    branchTriggerRef?: React.RefObject<HTMLButtonElement | null>;
  }) => (
    <div>
      <button ref={branchTriggerRef} data-testid="branch-trigger">
        branch trigger
      </button>
      <button
        data-testid="select-node"
        onClick={() => {
          onSelectionChange({ nodeIds: ['n1'], edgeIds: [], sourceWidgetId: undefined });
        }}
      >
        select
      </button>
      <span data-testid="reload-signal">{reloadSignal ?? 0}</span>
    </div>
  ),
}));
vi.mock('praxis/components/template-screen/properties-inspector', () => ({
  PropertiesInspector: ({
    selectionKind,
    selectionId,
    onSave,
    onReset,
  }: {
    selectionKind: string;
    selectionId?: string;
    onSave?: (payload: { name: string; dataSource: string }) => void;
    onReset?: () => void;
  }) => (
    <div data-testid="inspector">
      <div data-testid="selection-kind">
        {selectionKind}:{selectionId ?? ''}
      </div>
      <button
        data-testid="inspector-save"
        onClick={() => onSave?.({ name: 'Node name', dataSource: 'ds1' })}
      >
        save
      </button>
      <button data-testid="inspector-reset" onClick={() => onReset?.()}>
        reset
      </button>
    </div>
  ),
}));
vi.mock('aideon/shell/aideon-desktop-shell', () => ({
  AideonDesktopShell: ({
    navigation,
    toolbar,
    content,
    inspector,
  }: {
    navigation: React.ReactNode;
    toolbar: React.ReactNode;
    content: React.ReactNode;
    inspector: React.ReactNode;
  }) => (
    <div data-testid="layout">
      <div data-testid="toolbar">{toolbar}</div>
      <div data-testid="sidebar">{navigation}</div>
      <div data-testid="content">{content}</div>
      <div data-testid="inspector">{inspector}</div>
    </div>
  ),
}));
vi.mock('praxis/components/debug-overlay', () => ({
  DebugOverlay: ({ visible }: { visible: boolean }) =>
    visible ? <div data-testid="debug">debug</div> : undefined,
}));
vi.mock('praxis/templates', async () => {
  const actual = await vi.importActual('praxis/templates');
  return {
    ...actual,
    BUILT_IN_TEMPLATES: [{ id: 't1', name: 'Template 1', description: '', widgets: [] }],
  };
});

const registryMock = vi.fn(() => [
  { id: 'w1', label: 'Graph', defaultView: { kind: 'graph' }, defaultSize: 'full', type: 'graph' },
]);
vi.mock('praxis/widgets/registry', () => ({
  listWidgetRegistry: () => registryMock(),
}));
vi.mock('praxis/praxis-api', () => {
  const listScenarios = vi
    .fn()
    .mockResolvedValue([
      { id: 's1', name: 'Scenario 1', branch: 'main', updatedAt: '', isDefault: true },
    ]);
  const applyOperations = vi.fn().mockResolvedValue(undefined);
  return { listScenarios, applyOperations };
});
vi.mock('praxis/domain-data', () => ({
  listProjectsWithScenarios: vi.fn().mockResolvedValue([
    {
      id: 'p1',
      name: 'Proj',
      scenarios: [{ id: 's1', name: 'Scenario 1', branch: 'main', updatedAt: '', isDefault: true }],
    },
  ]),
  listTemplatesFromHost: vi
    .fn()
    .mockResolvedValue([{ id: 't1', name: 'Template 1', description: '', widgets: [] }]),
}));
vi.mock('praxis/platform', () => ({ isTauri: vi.fn(() => false) }));
const useTemporalPanelMock = vi.hoisted(() =>
  vi.fn<() => [TemporalPanelState, TemporalPanelActions]>(),
);
vi.mock('praxis/time/use-temporal-panel', () => ({
  useTemporalPanel: () => useTemporalPanelMock(),
}));
const commandStackMock = vi.hoisted(() => ({
  record: vi.fn(),
  undo: vi.fn(),
  redo: vi.fn(),
}));
vi.mock('praxis/hooks/use-command-stack', () => ({
  useCommandStack: () => commandStackMock,
}));
vi.mock('praxis/lib/analytics', () => ({ track: vi.fn() }));

import { listProjectsWithScenarios, listTemplatesFromHost } from 'praxis/domain-data';
import { PraxisWorkspaceSurface } from 'praxis/workspace';

describe('PraxisWorkspaceSurface (coverage)', () => {
  const baseTemporalState: TemporalPanelState = {
    branches: [],
    commits: [],
    loading: false,
    snapshotLoading: false,
    merging: false,
  };

  const baseTemporalActions: TemporalPanelActions = {
    refreshBranches: vi.fn().mockResolvedValue(undefined),
    selectCommit: vi.fn(),
    selectBranch: vi.fn().mockResolvedValue(undefined),
    mergeIntoMain: vi.fn().mockResolvedValue(undefined),
  };

  afterEach(() => {
    cleanup();
  });

  it('loads projects/templates, wires selection and saves templates', async () => {
    useTemporalPanelMock.mockReturnValue([
      {
        ...baseTemporalState,
        branch: 'main',
        commitId: undefined,
      },
      baseTemporalActions,
    ]);

    const onSelectionChange = vi.fn();
    render(<PraxisWorkspaceSurface onSelectionChange={onSelectionChange} />);

    await waitFor(() => {
      expect(listTemplatesFromHost).toHaveBeenCalled();
    });
    expect(screen.getByTestId('sidebar')).toBeInTheDocument();

    fireEvent.click(screen.getByTestId('select-node'));
    expect(onSelectionChange).toHaveBeenCalledWith({
      nodeIds: ['n1'],
      edgeIds: [],
      sourceWidgetId: undefined,
    });

    fireEvent.click(screen.getByTestId('save-template'));
    await waitFor(() => {
      expect(listTemplatesFromHost).toHaveBeenCalledTimes(1);
    });
  });

  it('changes scenarios', async () => {
    useTemporalPanelMock.mockReturnValue([
      {
        ...baseTemporalState,
        branch: 'main',
        commitId: undefined,
      },
      baseTemporalActions,
    ]);
    render(<PraxisWorkspaceSurface />);
    await waitFor(() => {
      expect(listProjectsWithScenarios).toHaveBeenCalled();
    });
    fireEvent.click(screen.getAllByTestId('scenario-change')[0]!);
    expect(screen.getAllByTestId('reload-signal')[0]!).toHaveTextContent('0');
  });

  it('opens widget library and creates widgets; handles empty registry', async () => {
    useTemporalPanelMock.mockReturnValue([
      {
        ...baseTemporalState,
        branch: 'main',
        commitId: undefined,
      },
      baseTemporalActions,
    ]);
    render(<PraxisWorkspaceSurface />);
    await waitFor(() => expect(screen.getAllByTestId('layout')[0]!).toBeInTheDocument());

    fireEvent.click(screen.getAllByTestId('open-library')[0]!);
    const widgetButton = await screen.findByText('Graph');
    fireEvent.click(widgetButton);

    registryMock.mockReturnValueOnce([]);
    fireEvent.click(screen.getAllByTestId('open-library')[0]!);
    expect(await screen.findByText(/No widget types registered/)).toBeInTheDocument();
  });

  it('invokes inspector save and applies operations for node selection', async () => {
    useTemporalPanelMock.mockReturnValue([
      {
        ...baseTemporalState,
        branch: 'main',
        commitId: 'c1',
        commits: [
          { id: 'c1', branch: 'main', parents: [], message: 'm', tags: [], changeCount: 0 },
        ],
      },
      baseTemporalActions,
    ]);
    const { applyOperations } = await import('praxis/praxis-api');
    render(<PraxisWorkspaceSurface />);
    await waitFor(() => {
      expect(listProjectsWithScenarios).toHaveBeenCalled();
    });
    fireEvent.click(screen.getAllByTestId('select-node')[0]!);
    const inspectorSaveButton = (await screen.findAllByTestId('inspector-save'))[0]!;
    fireEvent.click(inspectorSaveButton);
    await waitFor(() => {
      expect(applyOperations).toHaveBeenCalled();
    });
  });

  it('covers error paths and keyboard shortcuts', async () => {
    const temporalSelectCommit = vi.fn();
    useTemporalPanelMock.mockReturnValue([
      {
        ...baseTemporalState,
        branch: 'main',
        commitId: 'c1',
        commits: [
          { id: 'c0', branch: 'main', parents: [], message: 'c0', tags: [], changeCount: 0 },
          { id: 'c1', branch: 'main', parents: [], message: 'c1', tags: [], changeCount: 0 },
          { id: 'c2', branch: 'main', parents: [], message: 'c2', tags: [], changeCount: 0 },
        ],
      },
      {
        ...baseTemporalActions,
        selectCommit: temporalSelectCommit,
      },
    ]);

    vi.mocked(listTemplatesFromHost).mockRejectedValueOnce(new Error('templates down'));
    vi.mocked(listProjectsWithScenarios).mockRejectedValueOnce(new Error('projects down'));

    render(<PraxisWorkspaceSurface />);
    await waitFor(() => {
      expect(listTemplatesFromHost).toHaveBeenCalled();
    });

    fireEvent.keyDown(window, { key: 'z', ctrlKey: true });
    fireEvent.keyDown(window, { key: 'z', ctrlKey: true, shiftKey: true });
    fireEvent.keyDown(window, { key: 'y', ctrlKey: true });
    expect(commandStackMock.undo).toHaveBeenCalled();
    expect(commandStackMock.redo).toHaveBeenCalled();

    fireEvent.keyDown(window, { key: 'ArrowRight' });
    expect(temporalSelectCommit).toHaveBeenCalledWith('c2');

    fireEvent.keyDown(window, { key: 'ArrowLeft' });
    expect(temporalSelectCommit).toHaveBeenCalledWith('c0');

    fireEvent.keyDown(window, { key: ' ', shiftKey: true });
    expect(screen.getByTestId('branch-trigger')).toHaveFocus();
  });

  // negative paths covered in dedicated files; keep this focused on happy-path wiring
});
