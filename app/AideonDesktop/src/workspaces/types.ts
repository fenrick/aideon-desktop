import type { ComponentType } from 'react';

export type WorkspaceId = 'praxis' | 'metis' | 'mneme';

export interface WorkspaceModule {
  id: WorkspaceId;
  label: string;
  enabled: boolean;

  // these map 1:1 into AideonDesktopShell slots
  Navigation: ComponentType;
  Toolbar?: ComponentType;
  Content: ComponentType;
  Inspector?: ComponentType;
}
