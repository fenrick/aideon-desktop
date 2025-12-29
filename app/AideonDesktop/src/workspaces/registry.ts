import type { WorkspaceModule } from './types';
import { PRAXIS_WORKSPACE } from './praxis/module';

export const WORKSPACES: WorkspaceModule[] = [
  PRAXIS_WORKSPACE,
  {
    id: 'metis',
    label: 'Metis',
    enabled: false,
    Navigation: () => null,
    Content: () => null,
    Inspector: () => null,
  },
  {
    id: 'mneme',
    label: 'Mneme',
    enabled: false,
    Navigation: () => null,
    Content: () => null,
    Inspector: () => null,
  },
];

export function getWorkspace(id: WorkspaceModule['id']): WorkspaceModule {
  return WORKSPACES.find((workspace) => workspace.id === id) ?? WORKSPACES[0];
}

export function getWorkspaceOptions(): { id: string; label: string; disabled: boolean }[] {
  return WORKSPACES.map((workspace) => ({
    id: workspace.id,
    label: workspace.label,
    disabled: !workspace.enabled,
  }));
}
