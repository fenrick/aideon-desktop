import type { ComponentType, ReactNode } from 'react';
import { useCallback, useState } from 'react';

import { AideonDesktopShell } from 'aideon/shell/aideon-desktop-shell';
import { PraxisWorkspaceProvider } from './workspaces/praxis/workspace';
import { getWorkspace, getWorkspaceOptions } from './workspaces/registry';
import type { WorkspaceId } from './workspaces/types';

type WorkspaceSwitcherConfig = {
  currentId: string;
  options: { id: string; label: string; disabled: boolean }[];
  onSelect?: (workspaceId: string) => void;
};

type WorkspaceProviderProps = {
  children: ReactNode;
  workspaceSwitcher: WorkspaceSwitcherConfig;
};

function WorkspaceProviderPassthrough({ children }: WorkspaceProviderProps) {
  return <>{children}</>;
}

/**
 * Application root that hosts the active workspace inside the desktop shell.
 */
export function AideonDesktopRoot() {
  const [activeWorkspaceId, setActiveWorkspaceId] = useState('praxis');
  const workspace = getWorkspace(activeWorkspaceId);
  const workspaceOptions = getWorkspaceOptions();

  const handleWorkspaceSelect = useCallback((workspaceId: string) => {
    const resolved = getWorkspace(workspaceId as WorkspaceId);
    setActiveWorkspaceId(resolved.id);
  }, []);

  const WorkspaceProvider =
    workspace.id === 'praxis'
      ? (PraxisWorkspaceProvider as ComponentType<WorkspaceProviderProps>)
      : WorkspaceProviderPassthrough;

  const WorkspaceNavigation = workspace.Navigation;
  const WorkspaceToolbar = workspace.Toolbar;
  const WorkspaceContent = workspace.Content;
  const WorkspaceInspector = workspace.Inspector ?? (() => null);

  return (
    <WorkspaceProvider
      key={workspace.id}
      workspaceSwitcher={{
        currentId: workspace.id,
        options: workspaceOptions,
        onSelect: handleWorkspaceSelect,
      }}
    >
      <AideonDesktopShell
        toolbar={WorkspaceToolbar ? <WorkspaceToolbar /> : undefined}
        navigation={<WorkspaceNavigation />}
        content={<WorkspaceContent />}
        inspector={<WorkspaceInspector />}
      />
    </WorkspaceProvider>
  );
}
