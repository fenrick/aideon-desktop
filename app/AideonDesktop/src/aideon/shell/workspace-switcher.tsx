import type { ReactElement } from 'react';

import { CheckIcon, LayoutGridIcon } from 'lucide-react';

import { Button } from 'design-system/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from 'design-system/components/ui/dropdown-menu';
import { cn } from 'design-system/lib/utilities';

export interface WorkspaceOption {
  readonly id: string;
  readonly label: string;
  readonly disabled?: boolean;
}

export interface WorkspaceSwitcherProperties {
  readonly currentId: string;
  readonly options: readonly WorkspaceOption[];
  readonly onSelect?: (workspaceId: string) => void;
  readonly className?: string;
}

/**
 * Shell-level workspace switcher. Modules can supply their own routing later; today it is mostly a UX placeholder.
 * @param root0 - Component props.
 * @param root0.currentId - Current workspace id.
 * @param root0.options - Available workspaces.
 * @param root0.onSelect - Selection callback.
 * @param root0.className - Optional class.
 * @returns Workspace dropdown control.
 */
export function WorkspaceSwitcher({
  currentId,
  options,
  onSelect,
  className,
}: WorkspaceSwitcherProperties): ReactElement {
  const current = options.find((opt) => opt.id === currentId)?.label ?? 'Workspace';

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="outline"
          size="sm"
          className={cn('h-8 gap-2 rounded-full', className)}
          aria-label="Switch workspace"
        >
          <LayoutGridIcon className="size-4 text-muted-foreground" />
          <span className="max-w-[140px] truncate">{current}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-[220px]">
        <DropdownMenuLabel>Workspaces</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {options.map((opt) => (
          <DropdownMenuItem
            key={opt.id}
            disabled={opt.disabled}
            onSelect={() => {
              onSelect?.(opt.id);
            }}
          >
            <span className="flex-1">{opt.label}</span>
            {opt.id === currentId ? (
              <CheckIcon className="size-4 text-muted-foreground" />
            ) : undefined}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
