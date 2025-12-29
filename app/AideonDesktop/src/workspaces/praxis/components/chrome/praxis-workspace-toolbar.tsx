import { createPortal } from 'react-dom';
import { useState, type ReactNode } from 'react';

import { isTauri } from 'praxis/platform';
import type { CanvasTemplate } from 'praxis/templates';
import type { TemporalPanelActions, TemporalPanelState } from 'praxis/time/use-temporal-panel';

import { Button } from 'design-system/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from 'design-system/components/ui/dropdown-menu';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from 'design-system/components/ui/select';
import { cn } from 'design-system/lib/utilities';
import {
  Command,
  Clock,
  LayoutGrid,
  MoreHorizontal,
  RefreshCw,
} from 'lucide-react';

import { TimeControlPanel } from '../blocks/time-control-panel';

export interface PraxisWorkspaceToolbarProperties {
  readonly scenarioName?: string;
  readonly templates: CanvasTemplate[];
  readonly activeTemplateId: string;
  readonly templateName?: string;
  readonly onTemplateChange: (templateId: string) => void;
  readonly onTemplateSave: () => void;
  readonly onCreateWidget: () => void;
  readonly temporalState: TemporalPanelState;
  readonly temporalActions: TemporalPanelActions;
  readonly loading?: boolean;
}

const COMMAND_PALETTE_EVENT = 'aideon.workspace.open-command-palette';
const HEADER_PAGES = ['Canvas', 'Overview', 'Timeline', 'Activity'] as const;

/**
 * Compact page header for the Praxis workspace, with template selection and actions.
 * @param props - Workspace toolbar props.
 */
export function PraxisWorkspaceToolbar({
  scenarioName,
  templates,
  activeTemplateId,
  templateName,
  onTemplateChange,
  onTemplateSave,
  onCreateWidget,
  temporalState,
  temporalActions,
  loading = false,
}: PraxisWorkspaceToolbarProperties) {
  const [timeDialogOpen, setTimeDialogOpen] = useState(false);
  const [pagesDialogOpen, setPagesDialogOpen] = useState(false);
  const shouldUseNativeSelect = isTauri();

  const headerTitle = scenarioName ?? 'Mainline FY25';
  const headerSubtitle = templateName ?? 'Executive overview';
  const activeTemplateExists = templates.some((template) => template.id === activeTemplateId);
  const templateSelectValue = activeTemplateExists ? activeTemplateId : '';

  const handlePageMenuSelect = () => {
    setPagesDialogOpen(true);
  };

  const handleTimeMenuSelect = () => {
    setTimeDialogOpen(true);
  };

  const handleCommandsMenuSelect = () => {
    if (typeof globalThis === 'undefined') {
      return;
    }
    globalThis.dispatchEvent(new CustomEvent(COMMAND_PALETTE_EVENT));
  };

  return (
    <>
      <div className="border-b border-border/70 bg-background/80 px-4 py-4">
        <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div>
            <h1 className="text-lg font-semibold leading-tight text-foreground">{headerTitle}</h1>
            <p className="text-sm text-muted-foreground">{headerSubtitle}</p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <div className="min-w-[200px]">
              {shouldUseNativeSelect ? (
                <select
                  aria-label="Select template"
                  value={templateSelectValue}
                  disabled={loading || templates.length === 0}
                  onChange={(event) => {
                    const nextValue = event.target.value;
                    if (nextValue) {
                      onTemplateChange(nextValue);
                    }
                  }}
                  className={cn(
                    'h-9 w-full rounded-md border border-input bg-background px-3 text-sm text-foreground shadow-xs outline-none transition-colors focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50',
                  )}
                >
                  <option value="" disabled>
                    Select template
                  </option>
                  {templates.map((template) => (
                    <option key={template.id} value={template.id}>
                      {template.name}
                    </option>
                  ))}
                </select>
              ) : (
                <Select
                  value={templateSelectValue}
                  disabled={loading || templates.length === 0}
                  onValueChange={(value) => {
                    onTemplateChange(value);
                  }}
                  aria-label="Select template"
                >
                  <SelectTrigger className="h-9 w-full">
                    <SelectValue placeholder="Select template" />
                  </SelectTrigger>
                  <SelectContent>
                    {templates.map((template) => (
                      <SelectItem key={template.id} value={template.id}>
                        <div className="flex flex-col">
                          <span className="font-medium">{template.name}</span>
                          <span className="text-xs text-muted-foreground">{template.description}</span>
                        </div>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            </div>
            <Button
              variant="default"
              size="sm"
              onClick={onCreateWidget}
              disabled={loading}
            >
              Add widget
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={onTemplateSave}
              disabled={loading}
            >
              Save
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon" className="h-9 w-9" aria-label="More workspace actions">
                  <MoreHorizontal />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-60 space-y-1">
                <DropdownMenuItem
                  onSelect={() => {
                    temporalActions.refreshBranches().catch(() => false);
                  }}
                  className="flex items-center gap-2"
                >
                  <RefreshCw className="size-4" />
                  Refresh
                </DropdownMenuItem>
                <DropdownMenuItem
                  onSelect={handlePageMenuSelect}
                  className="flex items-center gap-2"
                >
                  <LayoutGrid className="size-4" />
                  Pages
                </DropdownMenuItem>
                <DropdownMenuItem
                  onSelect={handleTimeMenuSelect}
                  className="flex items-center gap-2"
                >
                  <Clock className="size-4" />
                  Time
                </DropdownMenuItem>
                <DropdownMenuItem
                  onSelect={handleCommandsMenuSelect}
                  className="flex items-center gap-2"
                >
                  <Command className="size-4" />
                  Commands
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      </div>
      <PageOverlay
        open={timeDialogOpen}
        onClose={() => setTimeDialogOpen(false)}
        title="Time controls"
        description="Manage branches, commits, and snapshots."
      >
        <TimeControlPanel state={temporalState} actions={temporalActions} />
      </PageOverlay>
      <PageOverlay
        open={pagesDialogOpen}
        onClose={() => setPagesDialogOpen(false)}
        title="Pages"
        description="Jump to a workspace page."
      >
        <div className="space-y-2 text-sm text-foreground">
          {HEADER_PAGES.map((page) => (
            <div key={page} className="rounded-md border border-border/50 px-3 py-2">
              {page}
            </div>
          ))}
        </div>
      </PageOverlay>
    </>
  );
}

function PageOverlay({
  open,
  onClose,
  title,
  description,
  children,
}: {
  readonly open: boolean;
  readonly onClose: () => void;
  readonly title: string;
  readonly description: string;
  readonly children: ReactNode;
}) {
  if (!open || typeof document === 'undefined') {
    return null;
  }

  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      onClick={onClose}
    >
      <div
        className="h-full w-full max-w-3xl overflow-auto rounded-2xl bg-background p-6 shadow-2xl"
        style={{ maxHeight: '90vh' }}
        onClick={(event) => {
          event.stopPropagation();
        }}
      >
        <div className="flex items-start justify-between gap-4">
          <div>
            <p className="text-sm font-semibold">{title}</p>
            <p className="text-xs text-muted-foreground">{description}</p>
          </div>
          <Button variant="ghost" size="sm" onClick={onClose}>
            Close
          </Button>
        </div>
        <div className="mt-4 text-xs/relaxed">{children}</div>
      </div>
    </div>,
    document.body,
  );
}
