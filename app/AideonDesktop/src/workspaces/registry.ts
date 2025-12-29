import type { WorkspaceModule } from './types';
import { METIS_WORKSPACE } from './metis/module';
import { MNEME_WORKSPACE } from './mneme/module';
import { PRAXIS_WORKSPACE } from './praxis/module';

export const WORKSPACES: WorkspaceModule[] = [
  PRAXIS_WORKSPACE,
  METIS_WORKSPACE,
  MNEME_WORKSPACE,
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
