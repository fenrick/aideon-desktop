import type { Ref } from 'react';
import { useMemo, useState } from 'react';

import type { TemporalPanelActions, TemporalPanelState } from 'praxis/time/use-temporal-panel';

import { searchStore } from 'praxis/lib/search';
import { isTauri } from 'praxis/platform';
import type { CanvasTemplate } from 'praxis/templates';

import { AideonToolbar } from 'aideon/shell/aideon-toolbar';
import { WorkspaceSwitcher } from 'aideon/shell/workspace-switcher';
import { Button } from 'design-system/components/ui/button';
import { Input } from 'design-system/components/ui/input';
import { Popover, PopoverContent, PopoverTrigger } from 'design-system/components/ui/popover';
import { toast } from 'sonner';
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
  readonly error?: string;
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
 * @param root0.error
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
  error,
}: PraxisWorkspaceToolbarProperties) {
  const isDesktop = isTauri();
  const modeLabel = isDesktop ? 'Desktop' : 'Browser preview';
  const trimmedScenarioName = scenarioName?.trim();
  const scenarioLabel =
    trimmedScenarioName && trimmedScenarioName.length > 0 ? trimmedScenarioName : 'Scenario';

  const [query, setQuery] = useState('');
  const placeholder = useMemo(() => {
    return `Search ${scenarioLabel.toLowerCase()}…`;
  }, [scenarioLabel]);

  return (
    <AideonToolbar
      title="Aideon"
      subtitle={scenarioName ? `Praxis · ${scenarioName}` : 'Praxis workspace'}
      modeLabel={modeLabel}
      statusMessage={error}
      onShellCommand={(command, payload) => {
        if (command === 'file.open' || command === 'file.save_as') {
          const path = (payload as { path?: unknown } | undefined)?.path;
          if (typeof path === 'string' && path.trim()) {
            toast.message(command === 'file.open' ? 'Selected file' : 'Save location', {
              description: path,
            });
          }
        }
      }}
      start={
        <WorkspaceSwitcher
          currentId="praxis"
          options={[
            { id: 'praxis', label: 'Praxis', disabled: false },
            { id: 'chrona', label: 'Chrona', disabled: true },
            { id: 'metis', label: 'Metis', disabled: true },
            { id: 'continuum', label: 'Continuum', disabled: true },
          ]}
        />
      }
      center={
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
      }
      end={
        <>
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
        </>
      }
    />
  );
}
