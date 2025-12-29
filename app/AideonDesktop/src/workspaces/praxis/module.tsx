import type { WorkspaceModule } from 'workspaces/types';

import {
  PraxisWorkspaceContent,
  PraxisWorkspaceInspector,
  PraxisWorkspaceNavigation,
  PraxisWorkspaceToolbar,
} from './workspace';

export const PRAXIS_WORKSPACE: WorkspaceModule = {
  id: 'praxis',
  label: 'Praxis',
  enabled: true,
  Navigation: PraxisWorkspaceNavigation,
  Toolbar: PraxisWorkspaceToolbar,
  Content: PraxisWorkspaceContent,
  Inspector: PraxisWorkspaceInspector,
};
