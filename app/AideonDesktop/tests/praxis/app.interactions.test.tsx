import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { SelectionState } from 'aideon/canvas/types';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const undo = vi.fn();
const redo = vi.fn();
const selectCommit = vi.fn();
const resetProperties = vi.fn();

vi.mock('praxis/stores/selection-store', () => {
  const selection = { nodeIds: ['n1'], edgeIds: [], sourceWidgetId: 'w1' };
  const properties = { n1: { name: 'Node', dataSource: 'ds', layout: 'grid', description: 'd' } };
  return {
    SelectionProvider: ({ children }: { children: ReactNode }) => (
      <span data-testid="selection-provider">{children}</span>
    ),
    useSelectionStore: () => ({
      state: { selection, properties },
      setSelection: vi.fn(),
      setFromWidget: vi.fn(),
      clear: vi.fn(),
      updateProperties: vi.fn(),
      resetProperties,
    }),
    deriveSelectionKind: vi.fn(() => 'node'),
    primarySelectionId: vi.fn(() => 'n1'),
  };
});

vi.mock('praxis/hooks/use-command-stack', () => ({
  useCommandStack: () => ({
    record: vi.fn(),
    undo,
    redo,
  }),
}));

vi.mock('praxis/time/use-temporal-panel', () => ({
  useTemporalPanel: () => [
    {
      branches: [{ name: 'main', head: 'c2' }],
      branch: 'main',
      commits: [
        { id: 'c1', message: 'first', parents: [], branch: 'main', tags: [], changeCount: 1 },
        { id: 'c2', message: 'second', parents: ['c1'], branch: 'main', tags: [], changeCount: 1 },
      ],
      commitId: 'c1',
      loading: false,
      snapshotLoading: false,
      merging: false,
    },
    {
      refreshBranches: vi.fn(() => Promise.resolve()),
      selectCommit,
      selectBranch: vi.fn(() => Promise.resolve()),
      mergeIntoMain: vi.fn(() => Promise.resolve()),
    },
  ],
}));

const templateError = vi.fn<() => boolean>();
const projectError = vi.fn<() => boolean>();

const listProjectsWithScenariosMock = vi.hoisted(() => vi.fn(() => Promise.resolve()));
const listTemplatesFromHostMock = vi.hoisted(() => vi.fn(() => Promise.resolve()));

afterEach(() => {
  cleanup();
});

vi.mock('praxis/domain-data', () => ({
  listProjectsWithScenarios: () =>
    listProjectsWithScenariosMock().then(() => {
      const shouldFail = projectError();
      if (shouldFail) {
        throw new Error('projects-failed');
      }
      return [
        {
          id: 'p1',
          name: 'Proj',
          scenarios: [
            { id: 's1', name: 'Scenario', branch: 'main', updatedAt: '', isDefault: true },
          ],
        },
      ];
    }),
  listTemplatesFromHost: () =>
    listTemplatesFromHostMock().then(() => {
      const shouldFail = templateError();
      if (shouldFail) {
        throw new Error('templates-failed');
      }
      return [{ id: 't1', documentId: 'canvasdoc-t1', name: 'Template 1', description: '', widgets: [] }];
    }),
}));

const templateSpy = vi.fn<(templates: { id: string; name: string }[]) => void>();

vi.mock('praxis/templates', () => ({
  BUILT_IN_TEMPLATES: [
    { id: 'fallback', documentId: 'canvasdoc-fallback', name: 'Fallback', description: 'built-in', widgets: [] },
  ],
  instantiateTemplate: vi.fn(() => []),
  captureTemplateFromWidgets: vi.fn((name: string) => ({
    id: `${name}-id`,
    documentId: `${name}-doc`,
    name,
    description: '',
    widgets: [],
  })),
}));

vi.mock('praxis/widgets/registry', () => ({
  listWidgetRegistry: () => [],
}));

vi.mock('praxis/praxis-api', () => ({
  applyOperations: vi.fn(() => Promise.resolve()),
}));

vi.mock('praxis/components/template-screen/projects-sidebar', () => ({
  ProjectsSidebar: ({
    projects,
    error,
    onRetry,
  }: {
    projects: unknown[];
    error?: string;
    onRetry?: () => void;
  }) => (
    <div>
      <span data-testid="project-count">{projects.length}</span>
      {error ? <p data-testid="projects-error">{error}</p> : undefined}
      <button data-testid="retry-projects" onClick={() => onRetry?.()}>
        retry
      </button>
    </div>
  ),
}));

vi.mock('praxis/components/chrome/praxis-workspace-toolbar', () => ({
  PraxisWorkspaceToolbar: ({
    onTemplateChange,
    templates = [],
  }: {
    onTemplateChange?: (templateId: string) => void;
    templates?: { id: string; name: string }[];
  }) => {
    templateSpy(templates);
    return (
      <button data-testid="change-template" onClick={() => onTemplateChange?.('alt')}>
        change template
      </button>
    );
  },
}));

vi.mock('praxis/components/template-screen/overview-tabs', () => ({
  OverviewTabs: () => <div data-testid="overview" />,
}));

vi.mock('praxis/components/template-screen/properties-inspector', () => ({
  PropertiesInspector: ({
    selection,
    onReset,
  }: {
    selection: SelectionState;
    onReset?: () => void;
  }) => (
    <button
      data-testid="reset-properties"
      data-selection-count={selection.nodeIds.length + selection.edgeIds.length}
      onClick={() => onReset?.()}
    >
      reset
    </button>
  ),
}));

import { PraxisWorkspaceSurface } from 'praxis/workspace';

describe('PraxisWorkspaceSurface interactions', () => {
  beforeEach(() => {
    undo.mockClear();
    redo.mockClear();
    selectCommit.mockClear();
    resetProperties.mockClear();
    templateSpy.mockClear();
    listProjectsWithScenariosMock.mockClear();
    listTemplatesFromHostMock.mockClear();
    templateError.mockReturnValue(false);
    projectError.mockReturnValue(false);
  });

  it('falls back to built-in templates when host load fails', async () => {
    templateError.mockReturnValueOnce(true);
    render(<PraxisWorkspaceSurface />);

    await waitFor(() => {
      expect(templateSpy).toHaveBeenCalled();
    });
    const latestTemplates =
      (templateSpy.mock.calls.at(-1)?.[0] as { id: string; name: string }[] | undefined) ?? [];
    expect(latestTemplates[0]?.name).toBe('Fallback');
  });

  it('surfaces project load errors and allows retry', async () => {
    projectError.mockReturnValueOnce(true);
    render(<PraxisWorkspaceSurface />);

    await screen.findByText(/projects-failed/);
    fireEvent.click(screen.getByTestId('retry-projects'));
    await waitFor(() => {
      expect(listProjectsWithScenariosMock).toHaveBeenCalledTimes(2);
    });
  });

  it('handles keyboard shortcuts for undo/redo and commit navigation', async () => {
    const keyTarget = globalThis as unknown as Window;
    render(<PraxisWorkspaceSurface />);
    await waitFor(() => {
      expect(screen.getAllByTestId('project-count').length).toBeGreaterThan(0);
    });

    fireEvent.keyDown(keyTarget, { key: 'z', metaKey: true });
    fireEvent.keyDown(keyTarget, { key: 'z', ctrlKey: true, shiftKey: true });
    fireEvent.keyDown(keyTarget, { key: 'ArrowRight' });

    expect(undo).toHaveBeenCalled();
    expect(redo).toHaveBeenCalled();
    expect(selectCommit).toHaveBeenCalledWith('c2');
  });

  it('resets properties via inspector action', async () => {
    render(<PraxisWorkspaceSurface />);
    await waitFor(() => {
      expect(screen.getAllByTestId('project-count').length).toBeGreaterThan(0);
    });

    fireEvent.click(screen.getByTestId('reset-properties'));
    expect(resetProperties).toHaveBeenCalledWith('n1');
  });
});
