import { METIS_WORKSPACE } from './metis/module';
import { MNEME_WORKSPACE } from './mneme/module';
import { PRAXIS_WORKSPACE } from './praxis/module';
import type { WorkspaceModule, WorkspaceNavigationProperties } from './types';

export const WORKSPACES: WorkspaceModule[] = [PRAXIS_WORKSPACE, METIS_WORKSPACE, MNEME_WORKSPACE];

/**
 *
 * @param id
 */
export function getWorkspace(id: WorkspaceModule['id']): WorkspaceModule {
  return WORKSPACES.find((workspace) => workspace.id === id) ?? PRAXIS_WORKSPACE;
}

/**
 *
 */
export function getWorkspaceOptions(): WorkspaceNavigationProperties['workspaceOptions'] {
  return WORKSPACES.map((workspace) => ({
    id: workspace.id,
    label: workspace.label,
    disabled: !workspace.enabled,
  }));
}
