import type { ComponentType, ReactNode } from 'react';
import { useCallback, useState } from 'react';

import { AideonToolbar } from 'aideon/shell/aideon-toolbar';
import { AideonDesktopShell } from 'aideon/shell/aideon-desktop-shell';
import { WorkspaceSwitcher } from 'aideon/shell/workspace-switcher';
import { isTauriRuntime } from 'lib/runtime';
import { PraxisWorkspaceProvider } from './workspaces/praxis/workspace';
import { getWorkspace, getWorkspaceOptions } from './workspaces/registry';
import type { WorkspaceId } from './workspaces/types';

const ACTIVE_WORKSPACE_STORAGE_KEY = 'aideon.active-workspace';

type WorkspaceProviderProps = {
  children: ReactNode;
};

function WorkspaceProviderPassthrough({ children }: WorkspaceProviderProps) {
  return <>{children}</>;
}

function readStoredWorkspaceId(): WorkspaceId | null {
  if (typeof globalThis === 'undefined') {
    return null;
  }
  const storage = (globalThis as { localStorage?: Storage }).localStorage;
  if (!storage) {
    return null;
  }
  try {
    const stored = storage.getItem(ACTIVE_WORKSPACE_STORAGE_KEY);
    if (stored === 'praxis' || stored === 'metis' || stored === 'mneme') {
      return stored;
    }
  } catch {
    return null;
  }
  return null;
}

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
      ? (PraxisWorkspaceProvider as ComponentType<WorkspaceProviderProps>)
      : WorkspaceProviderPassthrough;

  const modeLabel = isTauriRuntime() ? 'Desktop' : 'Browser preview';
  const WorkspaceNavigation = ws.Navigation;
  const WorkspaceToolbar = ws.Toolbar;
  const WorkspaceContent = ws.Content;
  const WorkspaceInspector = ws.Inspector ?? EmptyInspector;

  return (
    <WorkspaceProvider key={ws.id}>
      <AideonDesktopShell
        navigation={<WorkspaceNavigation />}
        toolbar={
          <AideonToolbar
            title="Aideon"
            subtitle={`${ws.label} workspace`}
            modeLabel={modeLabel}
            start={
              <WorkspaceSwitcher
                currentId={ws.id}
                options={workspaceOptions}
                onSelect={handleWorkspaceSelect}
              />
            }
            workspaceToolbar={WorkspaceToolbar ? <WorkspaceToolbar /> : null}
          />
        }
        content={<WorkspaceContent />}
        inspector={ws.Inspector ? <WorkspaceInspector /> : <EmptyInspector />}
      />
    </WorkspaceProvider>
  );
}
