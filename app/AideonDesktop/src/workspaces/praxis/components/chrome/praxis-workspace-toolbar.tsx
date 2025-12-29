import type { Ref } from 'react';
import { useMemo, useState } from 'react';

import type { TemporalPanelActions, TemporalPanelState } from 'praxis/time/use-temporal-panel';

import { searchStore } from 'praxis/lib/search';
import type { CanvasTemplate } from 'praxis/templates';

import { cn } from 'design-system/lib/utilities';
import { Button } from 'design-system/components/ui/button';
import { Input } from 'design-system/components/ui/input';
import { Popover, PopoverContent, PopoverTrigger } from 'design-system/components/ui/popover';
import { TimeControlPanel } from '../blocks/time-control-panel';
import { TemplateToolbar } from './template-toolbar';
import { WorkspaceActions } from './workspace-actions';

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
  readonly timeTriggerRef?: Ref<HTMLButtonElement>;
  readonly loading?: boolean;
  readonly className?: string;
}

/**
 * Top toolbar for the Praxis workspace inside the Aideon shell.
 * @param root0
 * @param root0.scenarioName
 * @param root0.templates
 * @param root0.activeTemplateId
 * @param root0.templateName
 * @param root0.onTemplateChange
 * @param root0.onTemplateSave
 * @param root0.onCreateWidget
 * @param root0.temporalState
 * @param root0.temporalActions
 * @param root0.timeTriggerRef
 * @param root0.loading
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
  timeTriggerRef,
  loading = false,
  className,
}: PraxisWorkspaceToolbarProperties) {
  const [query, setQuery] = useState('');
  const placeholder = useMemo(() => 'Search scenario…', []);

  return (
    <div className={cn('flex w-full items-center gap-2', className)}>
      <div className="hidden min-w-0 flex-1 md:block">
        <Input
          type="search"
          aria-label="Search"
          placeholder={placeholder}
          value={query}
          onChange={(event) => {
            const value = event.target.value;
            setQuery(value);
            if (value.trim()) {
              searchStore.search(value);
            } else {
              searchStore.clear();
            }
          }}
          className="h-9 bg-background/80"
        />
      </div>
      <div className="flex items-center gap-2">
        <Popover>
          <PopoverTrigger asChild>
            <Button ref={timeTriggerRef} variant="outline" size="sm">
              Time
            </Button>
          </PopoverTrigger>
          <PopoverContent align="end" className="w-[380px] p-0">
            <TimeControlPanel state={temporalState} actions={temporalActions} />
          </PopoverContent>
        </Popover>

        <WorkspaceActions />

        <TemplateToolbar
          scenarioName={scenarioName}
          templateName={templateName}
          templates={templates}
          activeTemplateId={activeTemplateId}
          onTemplateChange={onTemplateChange}
          onTemplateSave={onTemplateSave}
          onCreateWidget={onCreateWidget}
          loading={loading}
        />
      </div>
    </div>
  );
}
