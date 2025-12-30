import { useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

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
import { Clock, Command, LayoutGrid, MoreHorizontal, RefreshCw } from 'lucide-react';

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
 *
 */
function dispatchCommandPaletteEvent() {
  if (typeof globalThis === 'undefined') {
    return;
  }
  globalThis.dispatchEvent(new CustomEvent(COMMAND_PALETTE_EVENT));
}

/**
 * Compact page header for the Praxis workspace, with template selection and actions.
 * @param props - Workspace toolbar props.
 * @param props.scenarioName
 * @param props.templates
 * @param props.activeTemplateId
 * @param props.templateName
 * @param props.onTemplateChange
 * @param props.onTemplateSave
 * @param props.onCreateWidget
 * @param props.temporalState
 * @param props.temporalActions
 * @param props.loading
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

  const headerEyebrow = scenarioName ?? 'Mainline FY25';
  const headerTitle = templateName ?? 'Executive overview';
  const headerDescription = 'Graph + KPI + catalogue snapshot for leadership reviews';
  const activeTemplateExists = templates.some((template) => template.id === activeTemplateId);
  const templateSelectValue = activeTemplateExists ? activeTemplateId : '';

  const handlePageMenuSelect = () => {
    setPagesDialogOpen(true);
  };

  const handleTimeMenuSelect = () => {
    setTimeDialogOpen(true);
  };

  const handleCommandsMenuSelect = dispatchCommandPaletteEvent;

  return (
    <>
      <div className="border-t border-border/60 bg-background/90 px-3 pb-3 pt-3 md:px-4">
        <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div className="space-y-1">
            <p className="text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground">
              {headerEyebrow}
            </p>
            <h1 className="text-xl font-semibold leading-tight text-foreground">{headerTitle}</h1>
            <p className="text-sm text-muted-foreground line-clamp-1">{headerDescription}</p>
          </div>
          <div className="flex flex-wrap items-center justify-start gap-2 lg:justify-end">
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
                          <span className="text-xs text-muted-foreground">
                            {template.description}
                          </span>
                        </div>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            </div>
            <Button variant="default" size="sm" onClick={onCreateWidget} disabled={loading}>
              Add widget
            </Button>
            <Button variant="secondary" size="sm" onClick={onTemplateSave} disabled={loading}>
              Save
            </Button>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-9 w-9"
                  aria-label="More workspace actions"
                >
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
        onClose={() => {
          setTimeDialogOpen(false);
        }}
        title="Time controls"
        description="Manage timelines, moments, and snapshots."
      >
        <TimeControlPanel state={temporalState} actions={temporalActions} />
      </PageOverlay>
      <PageOverlay
        open={pagesDialogOpen}
        onClose={() => {
          setPagesDialogOpen(false);
        }}
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

/**
 *
 * @param root0
 * @param root0.open
 * @param root0.onClose
 * @param root0.title
 * @param root0.description
 * @param root0.children
 */
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
    return;
  }

  return createPortal(
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <button
        type="button"
        className="absolute inset-0 bg-black/60"
        aria-label="Close overlay"
        onClick={onClose}
      />
      <dialog
        open
        className="relative h-full w-full max-w-3xl max-h-[90vh] overflow-auto rounded-2xl bg-background p-6 shadow-2xl"
        aria-label={title}
        onCancel={(event) => {
          event.preventDefault();
          onClose();
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
      </dialog>
    </div>,
    document.body,
  );
}
