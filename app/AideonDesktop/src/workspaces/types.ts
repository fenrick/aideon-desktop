import type { ReactNode } from 'react';

import type { WorkspaceSwitcherProperties } from 'aideon/shell/workspace-switcher';

export interface WorkspaceShellSlots {
  readonly toolbar?: ReactNode;
  readonly navigation: ReactNode;
  readonly content: ReactNode;
  readonly inspector: ReactNode;
  readonly overlays?: ReactNode;
}

export type WorkspaceSwitcherConfig = Pick<
  WorkspaceSwitcherProperties,
  'currentId' | 'options' | 'onSelect'
>;

export interface WorkspaceHostProps {
  readonly workspaceSwitcher: WorkspaceSwitcherConfig;
  readonly children: (slots: WorkspaceShellSlots) => ReactNode;
}

export type WorkspaceHost = (props: WorkspaceHostProps) => ReactNode;

export interface WorkspaceDefinition {
  readonly id: string;
  readonly label: string;
  readonly Host: WorkspaceHost;
}
