import type { ReactNode } from 'react';

import type { WorkspaceId, WorkspaceNavigationProperties } from 'workspaces/types';

import { Button } from 'design-system/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from 'design-system/components/ui/tooltip';
import { cn } from 'design-system/lib/utilities';
import { Brain, Database, LayoutGrid } from 'lucide-react';

const WORKSPACE_ICONS: Record<WorkspaceId, typeof LayoutGrid> = {
  praxis: LayoutGrid,
  metis: Brain,
  mneme: Database,
};

export interface AideonDesktopNavigationProperties extends Readonly<WorkspaceNavigationProperties> {
  readonly children: ReactNode;
  readonly className?: string;
}

/**
 * Host-level navigation wrapper with a workspace icon rail and contextual sidebar content.
 * @param root0
 * @param root0.activeWorkspaceId
 * @param root0.workspaceOptions
 * @param root0.onWorkspaceSelect
 * @param root0.children
 * @param root0.className
 */
export function AideonDesktopNavigation({
  activeWorkspaceId,
  workspaceOptions,
  onWorkspaceSelect,
  children,
  className,
}: AideonDesktopNavigationProperties) {
  return (
    <div className={cn('flex h-full', className)}>
      <TooltipProvider>
        <div className="flex w-14 flex-col items-center gap-3 border-r border-border/70 bg-sidebar px-2 py-3">
          {workspaceOptions.map((workspace) => {
            const Icon = WORKSPACE_ICONS[workspace.id];
            const active = workspace.id === activeWorkspaceId;
            return (
              <Tooltip key={workspace.id}>
                <TooltipTrigger asChild>
                  <Button
                    type="button"
                    size="icon"
                    variant={active ? 'secondary' : 'ghost'}
                    className={cn(
                      'h-10 w-10 rounded-xl',
                      workspace.disabled ? 'cursor-not-allowed opacity-50' : undefined,
                    )}
                    aria-label={workspace.label}
                    disabled={workspace.disabled}
                    onClick={() => {
                      onWorkspaceSelect(workspace.id);
                    }}
                  >
                    <Icon className="size-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="right">
                  {workspace.label}
                  {workspace.disabled ? ' (Coming soon)' : ''}
                </TooltipContent>
              </Tooltip>
            );
          })}
        </div>
      </TooltipProvider>
      <div className="flex min-w-0 flex-1">{children}</div>
    </div>
  );
}
