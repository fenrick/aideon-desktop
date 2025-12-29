import { useCallback, useState } from 'react';

import { AideonDesktopShell } from 'aideon/shell/aideon-desktop-shell';
import { listWorkspaceOptions, resolveWorkspace } from './workspaces/registry';

/**
 * Application root mounting the Praxis Scenario / Template experience.
 * The surface already renders inside the three-pane shell.
 */
export function AideonDesktopRoot() {
  const [activeWorkspaceId, setActiveWorkspaceId] = useState('praxis');
  const workspace = resolveWorkspace(activeWorkspaceId);
  const workspaceOptions = listWorkspaceOptions();

  const handleWorkspaceSelect = useCallback((workspaceId: string) => {
    const resolved = resolveWorkspace(workspaceId);
    setActiveWorkspaceId(resolved.id);
  }, []);

  const WorkspaceHost = workspace.Host;

  return (
    <WorkspaceHost
      key={workspace.id}
      workspaceSwitcher={{
        currentId: workspace.id,
        options: workspaceOptions,
        onSelect: handleWorkspaceSelect,
      }}
    >
      {(slots) => (
        <>
          <AideonDesktopShell
            toolbar={slots.toolbar}
            navigation={slots.navigation}
            content={slots.content}
            inspector={slots.inspector}
          />
          {slots.overlays}
        </>
      )}
    </WorkspaceHost>
  );
}
