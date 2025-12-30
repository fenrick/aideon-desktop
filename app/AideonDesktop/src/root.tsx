import type { ComponentType, ReactElement } from 'react';
import { useCallback, useState } from 'react';

import { AideonDesktopNavigation } from 'aideon/shell/aideon-desktop-navigation';
import { AideonDesktopShell } from 'aideon/shell/aideon-desktop-shell';
import { AideonToolbar } from 'aideon/shell/aideon-toolbar';
import { isTauriRuntime } from 'lib/runtime';
import { PraxisWorkspaceProvider } from './workspaces/praxis/workspace';
import { getWorkspace, getWorkspaceOptions } from './workspaces/registry';
import type { WorkspaceId } from './workspaces/types';

const ACTIVE_WORKSPACE_STORAGE_KEY = 'aideon.active-workspace';

interface WorkspaceProviderProperties {
  readonly children: ReactElement;
}

/**
 *
 * @param root0
 * @param root0.children
 */
function WorkspaceProviderPassthrough({ children }: WorkspaceProviderProperties): ReactElement {
  return children;
}

/**
 *
 */
function readStoredWorkspaceId(): WorkspaceId | undefined {
  if (typeof globalThis === 'undefined') {
    return undefined;
  }
  const storage = (globalThis as { localStorage?: Storage }).localStorage;
  if (!storage) {
    return undefined;
  }
  try {
    const stored = storage.getItem(ACTIVE_WORKSPACE_STORAGE_KEY);
    if (stored === 'praxis' || stored === 'metis' || stored === 'mneme') {
      return stored;
    }
  } catch {
    return undefined;
  }
  return undefined;
}

/**
 *
 * @param id
 */
function persistWorkspaceId(id: WorkspaceId) {
  if (typeof globalThis === 'undefined') {
    return;
  }
  const storage = (globalThis as { localStorage?: Storage }).localStorage;
  if (!storage) {
    return;
  }
  try {
    storage.setItem(ACTIVE_WORKSPACE_STORAGE_KEY, id);
  } catch {
    return;
  }
}

/**
 *
 */
function EmptyInspector() {
  return <div className="p-4 text-sm text-muted-foreground">No inspector available.</div>;
}

/**
 * Application root that hosts the active workspace inside the desktop shell.
 */
export function AideonDesktopRoot() {
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<WorkspaceId>(
    () => readStoredWorkspaceId() ?? 'praxis',
  );
  const ws = getWorkspace(activeWorkspaceId);
  const workspaceOptions = getWorkspaceOptions();

  const handleWorkspaceSelect = useCallback((workspaceId: string) => {
    const resolved = getWorkspace(workspaceId as WorkspaceId);
    setActiveWorkspaceId(resolved.id);
    persistWorkspaceId(resolved.id);
  }, []);

  const WorkspaceProvider =
    ws.id === 'praxis'
      ? (PraxisWorkspaceProvider as ComponentType<WorkspaceProviderProperties>)
      : WorkspaceProviderPassthrough;

  const modeLabel = isTauriRuntime() ? 'Desktop' : undefined;
  const WorkspaceNavigation = ws.Navigation;
  const WorkspaceToolbar = ws.Toolbar;
  const WorkspaceContent = ws.Content;
  const WorkspaceInspector = ws.Inspector ?? EmptyInspector;

  return (
    <WorkspaceProvider key={ws.id}>
      <AideonDesktopShell
        navigation={
          <AideonDesktopNavigation
            activeWorkspaceId={ws.id}
            workspaceOptions={workspaceOptions}
            onWorkspaceSelect={handleWorkspaceSelect}
          >
            <WorkspaceNavigation
              activeWorkspaceId={ws.id}
              workspaceOptions={workspaceOptions}
              onWorkspaceSelect={handleWorkspaceSelect}
            />
          </AideonDesktopNavigation>
        }
        toolbar={
          <AideonToolbar
            title="Aideon"
            subtitle={`${ws.label} workspace`}
            modeLabel={modeLabel}
            workspaceToolbar={WorkspaceToolbar ? <WorkspaceToolbar /> : undefined}
          />
        }
        content={<WorkspaceContent />}
        inspector={ws.Inspector ? <WorkspaceInspector /> : <EmptyInspector />}
      />
    </WorkspaceProvider>
  );
}
