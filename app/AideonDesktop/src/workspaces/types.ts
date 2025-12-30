import type { ComponentType } from 'react';

export type WorkspaceId = 'praxis' | 'metis' | 'mneme';

export interface WorkspaceNavigationProperties {
  readonly activeWorkspaceId: WorkspaceId;
  readonly onWorkspaceSelect: (workspaceId: WorkspaceId) => void;
  readonly workspaceOptions: readonly {
    readonly id: WorkspaceId;
    readonly label: string;
    readonly disabled: boolean;
  }[];
}

export interface WorkspaceModule {
  id: WorkspaceId;
  label: string;
  enabled: boolean;

  // these map 1:1 into AideonDesktopShell slots
  Navigation: ComponentType<WorkspaceNavigationProperties>;
  Toolbar?: ComponentType;
  Content: ComponentType;
  Inspector?: ComponentType;
}
