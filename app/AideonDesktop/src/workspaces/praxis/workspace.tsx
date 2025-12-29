import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
  type RefCallback,
} from 'react';

import { dedupeIds } from 'aideon/canvas/selection';
import { AideonDesktopShell } from 'aideon/shell/aideon-desktop-shell';
import { Badge } from 'design-system/components/ui/badge';
import { Button } from 'design-system/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from 'design-system/components/ui/dialog';
import { isDevelopmentBuild } from 'lib/runtime';
import { PraxisWorkspaceToolbar as PraxisWorkspaceToolbarChrome } from 'praxis/components/chrome/praxis-workspace-toolbar';
import { DebugOverlay } from 'praxis/components/debug-overlay';
import { OverviewTabs } from 'praxis/components/template-screen/overview-tabs';
import { ProjectsSidebar } from 'praxis/components/template-screen/projects-sidebar';
import {
  PropertiesInspector,
  type SelectionKind,
} from 'praxis/components/template-screen/properties-inspector';
import {
  listProjectsWithScenarios,
  listTemplatesFromHost,
  type ProjectSummary,
} from 'praxis/domain-data';
import { useCommandStack } from 'praxis/hooks/use-command-stack';
import { track } from 'praxis/lib/analytics';
import { toErrorMessage } from 'praxis/lib/errors';
import { applyOperations, type ScenarioSummary } from 'praxis/praxis-api';
import {
  BUILT_IN_TEMPLATES,
  captureTemplateFromWidgets,
  instantiateTemplate,
  type CanvasTemplate,
  type TemplateWidgetConfig,
} from 'praxis/templates';
import type {
  PraxisCanvasWidget as CanvasWidget,
  SelectionState,
  PraxisWidgetKind as WidgetKind,
} from 'praxis/types';
import { listWidgetRegistry, type WidgetRegistryEntry } from 'praxis/widgets/registry';
import type { WorkspaceSwitcherProperties } from 'aideon/shell/workspace-switcher';
import {
  SelectionProvider,
  deriveSelectionKind,
  primarySelectionId,
  useSelectionStore,
  type SelectionProperties,
} from './stores/selection-store';
import { useTemporalPanel } from './time/use-temporal-panel';

const debugEnabled = isDevelopmentBuild();

/**
 *
 * @param entry
 * @param widgetId
 */
function createTemplateWidget(entry: WidgetRegistryEntry, widgetId: string): TemplateWidgetConfig {
  const base = { id: widgetId, title: entry.label, size: entry.defaultSize };
  switch (entry.defaultView.kind) {
    case 'graph': {
      return { ...base, kind: 'graph', view: entry.defaultView };
    }
    case 'chart': {
      return { ...base, kind: 'chart', view: entry.defaultView };
    }
    case 'catalogue': {
      return { ...base, kind: 'catalogue', view: entry.defaultView };
    }
    case 'matrix': {
      return { ...base, kind: 'matrix', view: entry.defaultView };
    }
    default: {
      return { ...base, kind: 'chart', view: entry.defaultView };
    }
  }
}

interface ScenarioState {
  loading: boolean;
  error?: string;
  data: ScenarioSummary[];
}

type WorkspaceSwitcherConfig = Pick<
  WorkspaceSwitcherProperties,
  'currentId' | 'options' | 'onSelect'
>;

interface PraxisWorkspaceContextValue {
  readonly projectState: {
    loading: boolean;
    data: ProjectSummary[];
    error?: string;
  };
  readonly scenarioState: ScenarioState;
  readonly templatesState: {
    loading: boolean;
    data: CanvasTemplate[];
    error?: string;
  };
  readonly activeTemplateId: string;
  readonly activeScenarioId?: string;
  readonly templateName?: string;
  readonly scenarioName?: string;
  readonly widgets: CanvasWidget[];
  readonly selection: SelectionState;
  readonly selectedProperties: SelectionProperties[];
  readonly selectionKind?: string;
  readonly selectionId?: string;
  readonly propertyState: {
    saving: boolean;
    error?: string;
    reloadTick: number;
  };
  readonly temporalState: ReturnType<typeof useTemporalPanel>[0];
  readonly temporalActions: ReturnType<typeof useTemporalPanel>[1];
  readonly workspaceSwitcher?: WorkspaceSwitcherConfig;
  readonly branchSelectReferenceCallback: RefCallback<HTMLButtonElement>;
  readonly onTemplateChange: (templateId: string) => void;
  readonly onTemplateSave: () => void;
  readonly onCreateWidget: () => void;
  readonly onSelectScenario: (scenarioId: string) => void;
  readonly onRetryProjects: () => void;
  readonly onSelectionChange: (selection: SelectionState) => void;
  readonly onInspectorSave: (updates: SelectionProperties) => void;
  readonly onInspectorReset: () => void;
  readonly debugVisible: boolean;
  readonly widgetLibraryOpen: boolean;
  readonly onToggleWidgetLibrary: (open: boolean) => void;
  readonly onCreateWidgetType: (type: WidgetKind) => void;
}

const PraxisWorkspaceContext = createContext<PraxisWorkspaceContextValue | undefined>(undefined);

function usePraxisWorkspaceContext(): PraxisWorkspaceContextValue {
  const context = useContext(PraxisWorkspaceContext);
  if (!context) {
    throw new Error('Praxis workspace components must be rendered within PraxisWorkspaceProvider.');
  }
  return context;
}

/**
 * Entry point for the Praxis workspace renderer.
 * @returns {import('react').ReactElement} Workspace route content.
 */
export default function App() {
  return <PraxisWorkspaceSurface />;
}

/**
 * Exposes the canvas surface for embedding; forwards selection changes.
 * @param root0
 * @param root0.onSelectionChange
 */
export function PraxisWorkspaceSurface({
  onSelectionChange,
}: {
  readonly onSelectionChange?: (selection: SelectionState) => void;
} = {}) {
  return (
    <PraxisWorkspaceProvider onSelectionChange={onSelectionChange}>
      <AideonDesktopShell
        toolbar={<PraxisWorkspaceToolbar />}
        navigation={<PraxisWorkspaceNavigation />}
        content={<PraxisWorkspaceContent />}
        inspector={<PraxisWorkspaceInspector />}
      />
    </PraxisWorkspaceProvider>
  );
}

interface PraxisWorkspaceProviderProperties {
  readonly onSelectionChange?: (selection: SelectionState) => void;
  readonly workspaceSwitcher?: WorkspaceSwitcherConfig;
  readonly children: ReactNode;
}

export function PraxisWorkspaceProvider({
  onSelectionChange,
  workspaceSwitcher,
  children,
}: PraxisWorkspaceProviderProperties) {
  return (
    <SelectionProvider>
      <PraxisWorkspaceStateProvider
        onSelectionChange={onSelectionChange}
        workspaceSwitcher={workspaceSwitcher}
      >
        {children}
      </PraxisWorkspaceStateProvider>
    </SelectionProvider>
  );
}

/**
 *
 * @param root0
 * @param root0.onSelectionChange
 */
function PraxisWorkspaceStateProvider({
  onSelectionChange,
  workspaceSwitcher,
  children,
}: {
  readonly onSelectionChange?: (selection: SelectionState) => void;
  readonly workspaceSwitcher?: WorkspaceSwitcherConfig;
  readonly children: ReactNode;
}) {
  const {
    state: selectionState,
    setFromWidget,
    setSelection,
    clear,
    updateProperties,
    resetProperties,
  } = useSelectionStore();
  const [projectState, setProjectState] = useState<{
    loading: boolean;
    data: ProjectSummary[];
    error?: string;
  }>({
    loading: true,
    data: [],
  });
  const [scenarioState, setScenarioState] = useState<ScenarioState>({ loading: true, data: [] });
  const [templatesState, setTemplatesState] = useState<{
    loading: boolean;
    data: CanvasTemplate[];
    error?: string;
  }>({
    loading: true,
    data: [],
  });
  const [activeTemplateId, setActiveTemplateId] = useState<string>('');
  const [activeScenarioId, setActiveScenarioId] = useState<string | undefined>();
  const [widgetLibraryOpen, setWidgetLibraryOpen] = useState(false);
  const [propertyState, setPropertyState] = useState<{
    saving: boolean;
    error?: string;
    reloadTick: number;
  }>({
    saving: false,
    reloadTick: 0,
  });
  const [debugVisible, setDebugVisible] = useState(false);
  const branchSelectReference = useRef<HTMLButtonElement | undefined>(undefined);
  const branchSelectReferenceCallback: RefCallback<HTMLButtonElement> = useCallback((node) => {
    branchSelectReference.current = node ?? undefined;
  }, []);
  const commandStack = useCommandStack();

  const [temporalState, temporalActions] = useTemporalPanel();

  useEffect(() => {
    if (onSelectionChange) {
      onSelectionChange(selectionState.selection);
    }
  }, [onSelectionChange, selectionState.selection]);

  useEffect(() => {
    if (temporalState.branch || temporalState.commitId) {
      track('time.cursor', { branch: temporalState.branch, commitId: temporalState.commitId });
    }
  }, [temporalState.branch, temporalState.commitId]);

  const refreshTemplates = useCallback(async () => {
    setTemplatesState((previous) => ({ ...previous, loading: true, error: undefined }));
    try {
      const templates = await listTemplatesFromHost();
      setTemplatesState({ loading: false, data: templates });
      setActiveTemplateId((previous) => previous || (templates[0]?.id ?? ''));
    } catch (unknownError) {
      setTemplatesState({
        loading: false,
        data: BUILT_IN_TEMPLATES,
        error: toErrorMessage(unknownError),
      });
      setActiveTemplateId((previous) => previous || (BUILT_IN_TEMPLATES[0]?.id ?? ''));
    }
  }, []);

  const refreshProjects = useCallback(async () => {
    setProjectState((previous) => ({ ...previous, loading: true, error: undefined }));
    try {
      const projects = await listProjectsWithScenarios();
      const scenarios = projects.flatMap((project) => project.scenarios);
      setProjectState({ loading: false, data: projects });
      setScenarioState({ loading: false, data: scenarios });
      setActiveScenarioId((previous) => {
        if (previous) {
          return previous;
        }
        const defaultScenario = scenarios.find((scenario) => scenario.isDefault) ?? scenarios[0];
        return defaultScenario?.id;
      });
    } catch (unknownError) {
      const message = toErrorMessage(unknownError);
      setProjectState({ loading: false, data: [], error: message });
      setScenarioState({ loading: false, data: [], error: message });
    }
  }, []);

  useEffect(() => {
    refreshTemplates().catch((_ignoredError: unknown) => {
      return;
    });
    refreshProjects().catch((_ignoredError: unknown) => {
      return;
    });
  }, [refreshProjects, refreshTemplates]);

  const activeScenario = useMemo(() => {
    const preferred =
      scenarioState.data.find((scenario) => scenario.id === activeScenarioId) ??
      scenarioState.data.find((scenario) => scenario.isDefault);
    return preferred ?? scenarioState.data[0];
  }, [activeScenarioId, scenarioState.data]);

  const activeTemplate = useMemo(() => {
    return (
      templatesState.data.find((entry) => entry.id === activeTemplateId) ?? templatesState.data[0]
    );
  }, [activeTemplateId, templatesState.data]);

  const widgets = useMemo<CanvasWidget[]>(() => {
    if (!activeTemplate) {
      return [];
    }
    return instantiateTemplate(activeTemplate, { scenario: activeScenario?.branch });
  }, [activeScenario?.branch, activeTemplate]);

  const selectionKind = deriveSelectionKind(selectionState.selection);
  const selectionId = primarySelectionId(selectionState.selection);
  const selectedProperties = selectionId
    ? (Reflect.get(selectionState.properties, selectionId) as SelectionProperties | undefined)
    : undefined;

  const handleSelectionChange = useCallback(
    (next: SelectionState) => {
      const previous = selectionState.selection;
      const normalised: SelectionState = {
        sourceWidgetId: next.sourceWidgetId,
        nodeIds: dedupeIds(next.nodeIds),
        edgeIds: dedupeIds(next.edgeIds),
      };
      setSelection(normalised);
      commandStack.record({
        label: 'Selection change',
        redo: () => {
          setSelection(normalised);
        },
        undo: () => {
          setSelection(previous);
        },
      });
      track('selection.change', {
        kind: deriveSelectionKind(normalised),
        sourceWidgetId: normalised.sourceWidgetId,
        nodeCount: normalised.nodeIds.length,
        edgeCount: normalised.edgeIds.length,
      });
    },
    [commandStack, selectionState.selection, setSelection],
  );

  const handleTemplateChange = useCallback(
    (templateId: string) => {
      const previous = activeTemplateId;
      setActiveTemplateId(templateId);
      clear();
      commandStack.record({
        label: 'Template change',
        redo: () => {
          setActiveTemplateId(templateId);
        },
        undo: () => {
          setActiveTemplateId(previous);
        },
      });
      track('template.change', { templateId, scenarioId: activeScenario?.id });
    },
    [activeScenario?.id, activeTemplateId, clear, commandStack],
  );

  const handleTemplateSave = useCallback(() => {
    if (widgets.length === 0) {
      return;
    }
    const nextIndexLabel = (templatesState.data.length + 1).toString();
    const name = `Template ${nextIndexLabel}`;
    const snapshot = captureTemplateFromWidgets(name, 'Saved from runtime', widgets);
    setTemplatesState((previous) => ({ ...previous, data: [...previous.data, snapshot] }));
    setActiveTemplateId(snapshot.id);
    track('template.change', { templateId: snapshot.id, scenarioId: activeScenario?.id });
  }, [activeScenario?.id, templatesState.data.length, widgets]);

  const handleScenarioSelect = useCallback(
    (scenarioId: string) => {
      const previous = activeScenarioId;
      setActiveScenarioId(scenarioId);
      clear();
      commandStack.record({
        label: 'Scenario change',
        redo: () => {
          setActiveScenarioId(scenarioId);
        },
        undo: () => {
          setActiveScenarioId(previous);
        },
      });
    },
    [activeScenarioId, clear, commandStack],
  );

  const handleWidgetCreate = useCallback(
    (type: WidgetKind) => {
      if (!activeTemplate) {
        return;
      }
      const entry = listWidgetRegistry().find((item) => item.type === type);
      if (!entry) {
        return;
      }
      const newWidgetId = `${type}-${Date.now().toString(36)}`;
      const newWidget = createTemplateWidget(entry, newWidgetId);
      const newTemplate: CanvasTemplate = {
        ...activeTemplate,
        widgets: [...activeTemplate.widgets, newWidget],
      };
      setTemplatesState((previous) => ({
        ...previous,
        data: previous.data.map((template) =>
          template.id === activeTemplate.id ? newTemplate : template,
        ),
      }));
      setActiveTemplateId(newTemplate.id);
      setFromWidget({ widgetId: newWidgetId, nodeIds: [], edgeIds: [] });
      setWidgetLibraryOpen(false);
      track('template.create_widget', { widgetType: type, templateId: newTemplate.id });
    },
    [activeTemplate, setFromWidget],
  );

  const handleInspectorSave = useCallback(
    (patch: Record<string, string | undefined>) => {
      const id = selectionId;
      if (!id) {
        return;
      }
      const kind = selectionKind;
      updateProperties(id, {
        name: patch.name,
        dataSource: patch.dataSource,
        layout: patch.layout,
        description: patch.description,
      });
      setPropertyState((previous) => ({ ...previous, saving: true, error: undefined }));
      void (async () => {
        try {
          if (kind === 'node') {
            await applyOperations([
              {
                kind: 'updateNode',
                node: { id, props: { label: patch.name, dataSource: patch.dataSource } },
              },
            ]);
            setPropertyState((previous) => ({ ...previous, reloadTick: previous.reloadTick + 1 }));
          }
          track('inspector.save', { selectionKind: kind, selectionId: id });
        } catch (unknownError) {
          setPropertyState((previous) => ({ ...previous, error: toErrorMessage(unknownError) }));
          track('error.ui', { surface: 'inspector', message: toErrorMessage(unknownError) });
        } finally {
          setPropertyState((previous) => ({ ...previous, saving: false }));
        }
      })();
    },
    [selectionId, selectionKind, updateProperties],
  );

  const handleInspectorReset = useCallback(() => {
    if (selectionId) {
      resetProperties(selectionId);
    }
  }, [resetProperties, selectionId]);

  const handleUndoRedo = useCallback(
    (event: KeyboardEvent): boolean => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'z' && !event.shiftKey) {
        event.preventDefault();
        commandStack.undo();
        return true;
      }
      if (
        ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'z') ||
        event.key.toLowerCase() === 'y'
      ) {
        event.preventDefault();
        commandStack.redo();
        return true;
      }
      return false;
    },
    [commandStack],
  );

  const handleArrowNavigation = useCallback(
    (event: KeyboardEvent): boolean => {
      const offsets: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1 };
      const offset = offsets[event.key];
      if (offset === undefined) {
        return false;
      }
      const index = temporalState.commits.findIndex(
        (commit) => commit.id === temporalState.commitId,
      );
      const target = index === -1 ? undefined : temporalState.commits[index + offset];
      if (target) {
        temporalActions.selectCommit(target.id);
        track('time.cursor', { branch: temporalState.branch, commitId: target.id });
      }
      return true;
    },
    [temporalActions, temporalState.branch, temporalState.commitId, temporalState.commits],
  );

  const sliderFocusShortcut = useCallback((event: KeyboardEvent) => {
    if (event.key === ' ' && event.shiftKey) {
      branchSelectReference.current?.focus();
    }
  }, []);

  useEffect(() => {
    const handleKeydown = (event: KeyboardEvent) => {
      if (handleUndoRedo(event)) {
        return;
      }
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'd') {
        event.preventDefault();
        setDebugVisible((previous) => !previous);
      }
      if (handleArrowNavigation(event)) {
        return;
      }
      sliderFocusShortcut(event);
    };
    globalThis.addEventListener('keydown', handleKeydown);
    return () => {
      globalThis.removeEventListener('keydown', handleKeydown);
    };
  }, [handleArrowNavigation, handleUndoRedo, sliderFocusShortcut]);

  const handleRetryProjects = useCallback(() => {
    refreshProjects().catch((_ignoredError: unknown) => {
      return;
    });
  }, [refreshProjects]);

  const contextValue = useMemo<PraxisWorkspaceContextValue>(() => {
    return {
      projectState,
      scenarioState,
      templatesState,
      activeTemplateId: activeTemplate?.id ?? '',
      activeScenarioId: activeScenario?.id,
      templateName: activeTemplate?.name,
      scenarioName: activeScenario?.name,
      widgets,
      selection: selectionState.selection,
      selectedProperties,
      selectionKind,
      selectionId,
      propertyState,
      temporalState,
      temporalActions,
      workspaceSwitcher,
      branchSelectReferenceCallback,
      onTemplateChange: handleTemplateChange,
      onTemplateSave: handleTemplateSave,
      onCreateWidget: () => {
        setWidgetLibraryOpen(true);
      },
      onSelectScenario: handleScenarioSelect,
      onRetryProjects: handleRetryProjects,
      onSelectionChange: handleSelectionChange,
      onInspectorSave: handleInspectorSave,
      onInspectorReset: handleInspectorReset,
      debugVisible,
      widgetLibraryOpen,
      onToggleWidgetLibrary: setWidgetLibraryOpen,
      onCreateWidgetType: handleWidgetCreate,
    };
  }, [
    activeScenario?.id,
    activeScenario?.name,
    activeTemplate?.id,
    activeTemplate?.name,
    branchSelectReferenceCallback,
    debugVisible,
    handleInspectorReset,
    handleInspectorSave,
    handleRetryProjects,
    handleScenarioSelect,
    handleSelectionChange,
    handleTemplateChange,
    handleTemplateSave,
    handleWidgetCreate,
    projectState,
    scenarioState,
    selectionId,
    selectionKind,
    selectionState.selection,
    selectedProperties,
    templatesState,
    temporalActions,
    temporalState,
    widgetLibraryOpen,
    widgets,
    workspaceSwitcher,
  ]);

  return (
    <PraxisWorkspaceContext.Provider value={contextValue}>
      {children}
    </PraxisWorkspaceContext.Provider>
  );
}

export function PraxisWorkspaceNavigation() {
  const { projectState, scenarioState, activeScenarioId, onSelectScenario, onRetryProjects } =
    usePraxisWorkspaceContext();

  return (
    <ProjectsSidebar
      projects={projectState.data}
      scenarios={scenarioState.data}
      loading={projectState.loading}
      error={projectState.error}
      activeScenarioId={activeScenarioId}
      onSelectScenario={onSelectScenario}
      onRetry={onRetryProjects}
    />
  );
}

export function PraxisWorkspaceToolbar() {
  const {
    scenarioName,
    templateName,
    templatesState,
    activeTemplateId,
    onTemplateChange,
    onTemplateSave,
    onCreateWidget,
    temporalState,
    temporalActions,
    branchSelectReferenceCallback,
    scenarioState,
    workspaceSwitcher,
  } = usePraxisWorkspaceContext();

  return (
    <PraxisWorkspaceToolbarChrome
      scenarioName={scenarioName}
      templateName={templateName}
      templates={templatesState.data}
      activeTemplateId={activeTemplateId}
      onTemplateChange={onTemplateChange}
      onTemplateSave={onTemplateSave}
      onCreateWidget={onCreateWidget}
      temporalState={temporalState}
      temporalActions={temporalActions}
      timeTriggerRef={branchSelectReferenceCallback}
      loading={templatesState.loading}
      error={scenarioState.error}
      workspaceSwitcher={workspaceSwitcher}
    />
  );
}

export function PraxisWorkspaceContent() {
  const {
    temporalState,
    temporalActions,
    widgets,
    selection,
    onSelectionChange,
    branchSelectReferenceCallback,
    propertyState,
    debugVisible,
    scenarioName,
    templateName,
    widgetLibraryOpen,
    onToggleWidgetLibrary,
    onCreateWidgetType,
  } = usePraxisWorkspaceContext();

  return (
    <>
      <div className="space-y-6">
        <OverviewTabs
          state={temporalState}
          actions={temporalActions}
          widgets={widgets}
          selection={selection}
          onSelectionChange={onSelectionChange}
          onRequestMetaModelFocus={(types) => {
            if (types.length === 0) {
              return;
            }
          }}
          reloadSignal={propertyState.reloadTick}
          branchTriggerRef={branchSelectReferenceCallback}
        />
      </div>
      <DebugOverlay
        visible={debugVisible && debugEnabled}
        scenarioName={scenarioName}
        templateName={templateName}
        selection={selection}
        branch={temporalState.branch}
        commitId={temporalState.commitId}
      />
      <WidgetLibraryDialog
        open={widgetLibraryOpen}
        onOpenChange={onToggleWidgetLibrary}
        registry={listWidgetRegistry()}
        onCreate={onCreateWidgetType}
      />
    </>
  );
}

export function PraxisWorkspaceInspector() {
  const { selectionKind, selectionId, selectedProperties, propertyState, onInspectorSave, onInspectorReset } =
    usePraxisWorkspaceContext();

  return (
    <PropertiesInspector
      key={selectionId ?? 'none'}
      selectionKind={(selectionKind ?? 'none') as SelectionKind}
      selectionId={selectionId}
      properties={selectedProperties}
      onSave={onInspectorSave}
      onReset={onInspectorReset}
      saving={propertyState.saving}
      error={propertyState.error}
    />
  );
}

interface WidgetLibraryDialogProperties {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  readonly registry: WidgetRegistryEntry[];
  readonly onCreate: (type: WidgetKind) => void;
}

/**
 *
 * @param root0
 * @param root0.open
 * @param root0.onOpenChange
 * @param root0.registry
 * @param root0.onCreate
 */
function WidgetLibraryDialog({
  open,
  onOpenChange,
  registry,
  onCreate,
}: WidgetLibraryDialogProperties) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Add widget</DialogTitle>
          <DialogDescription>Choose a widget type to add to this template.</DialogDescription>
        </DialogHeader>
        <div className="space-y-2">
          {registry.map((entry) => (
            <button
              key={entry.type}
              type="button"
              className="flex w-full items-center justify-between rounded-lg border border-border/70 bg-card px-3 py-2 text-left hover:border-primary/50 hover:bg-muted/40"
              onClick={() => {
                onCreate(entry.type);
              }}
            >
              <div>
                <p className="text-sm font-semibold">{entry.label}</p>
                <p className="text-xs text-muted-foreground">{entry.description}</p>
              </div>
              <Badge variant="outline">{entry.defaultSize}</Badge>
            </button>
          ))}
          {registry.length === 0 ? (
            <p className="text-sm text-muted-foreground">No widget types registered.</p>
          ) : undefined}
        </div>
        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => {
              onOpenChange(false);
            }}
          >
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
