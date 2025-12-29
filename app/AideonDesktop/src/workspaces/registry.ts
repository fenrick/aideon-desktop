import type { WorkspaceOption } from 'aideon/shell/workspace-switcher';
import { PraxisWorkspaceHost } from './praxis/workspace';
import type { WorkspaceDefinition } from './types';

const workspaceRegistry: WorkspaceDefinition[] = [
  {
    id: 'praxis',
    label: 'Praxis',
    Host: PraxisWorkspaceHost,
  },
];

const workspaceOptions: WorkspaceOption[] = [
  { id: 'praxis', label: 'Praxis', disabled: false },
  { id: 'chrona', label: 'Chrona', disabled: true },
  { id: 'metis', label: 'Metis', disabled: true },
  { id: 'continuum', label: 'Continuum', disabled: true },
];

export function listWorkspaces(): WorkspaceDefinition[] {
  return workspaceRegistry;
}

export function listWorkspaceOptions(): WorkspaceOption[] {
  return workspaceOptions;
}

export function resolveWorkspace(id: string): WorkspaceDefinition {
  return workspaceRegistry.find((workspace) => workspace.id === id) ?? workspaceRegistry[0];
}
