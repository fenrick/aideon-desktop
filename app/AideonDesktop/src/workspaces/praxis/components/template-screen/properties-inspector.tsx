import type { ReactNode } from 'react';
import { useEffect, useMemo } from 'react';
import { useForm, type UseFormReturn } from 'react-hook-form';

import { templateScreenCopy } from 'praxis/copy/template-screen';
import type { SelectionProperties } from 'praxis/stores/selection-store';

import type { SelectionState } from 'aideon/canvas/types';

import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from 'design-system/components/ui/accordion';
import { Badge } from 'design-system/components/ui/badge';
import { Button } from 'design-system/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from 'design-system/components/ui/card';
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
} from 'design-system/components/ui/form';
import { Input } from 'design-system/components/ui/input';
import { ScrollArea } from 'design-system/components/ui/scroll-area';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from 'design-system/components/ui/select';
import { Switch } from 'design-system/components/ui/switch';
import { Textarea } from 'design-system/components/ui/textarea';

export type SelectionKind = 'widget' | 'node' | 'edge' | 'none';

/**
 * Local form values stored for the inspector form.
 */
interface FormValues {
  name: string;
  dataSource: string;
  layout: string;
  description: string;
  filter: string;
  displayMode: 'default' | 'compact' | 'expanded';
  showLabels: boolean;
  interactionMode: 'default' | 'advanced';
}

/**
 * Normalize a preferred value with an optional fallback.
 * @param primary - Preferred string value.
 * @param fallback - Value to use if the primary is missing.
 */
function resolveFieldValue(primary?: string, fallback?: string) {
  return primary ?? fallback ?? '';
}

/**
 * Placeholder for bulk actions, kept to avoid reintroducing command plumbing.
 * @param _label - Bulk action label (unused).
 */
function noopBulkAction(_label: string) {
  void 0;
}

/**
 * Stub handler for align bulk actions.
 */
function alignBulkAction() {
  noopBulkAction('Align');
}

/**
 * Stub handler for distribute bulk actions.
 */
function distributeBulkAction() {
  noopBulkAction('Distribute');
}

/**
 * Stub handler for delete bulk actions.
 */
function deleteBulkAction() {
  noopBulkAction('Delete');
}

interface MultiSelectionPanelProperties {
  readonly selectionCount: number;
}

/**
 * Panel showing the multi-selection summary.
 * @param root0 - Component props.
 * @param root0.selectionCount - Total number of selected items.
 */
function MultiSelectionPanel({ selectionCount }: MultiSelectionPanelProperties) {
  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">
        {selectionCount} widgets selected. Align, distribute, or delete them in one go.
      </p>
    </div>
  );
}

/**
 * Props passed to the widget form helper component.
 */
interface WidgetFormPanelProperties {
  readonly form: UseFormReturn<FormValues>;
  readonly copy: typeof templateScreenCopy.properties;
}

/**
 * Renders the accordion-backed form for a single widget.
 * @param root0 - Component props.
 * @param root0.form - Hook form instance.
 * @param root0.copy - Copy map with localized labels.
 */
/**
 * Data section inside the widget form accordion.
 * @param root0 - Section props.
 * @param root0.form - Hook form instance.
 * @param root0.copy - Copy translations.
 */
function DataSection({ form, copy }: WidgetFormPanelProperties) {
  return (
    <AccordionItem value="data" className="rounded-2xl border border-border/60">
      <AccordionTrigger>Data</AccordionTrigger>
      <AccordionContent className="space-y-4 pt-3">
        <FormField
          control={form.control}
          name="name"
          render={({ field }) => (
            <FormItem className="space-y-2">
              <FormLabel>{copy.nameLabel}</FormLabel>
              <FormControl>
                <Input placeholder="Widget name" {...field} />
              </FormControl>
              <FormDescription>Friendly label for the widget</FormDescription>
            </FormItem>
          )}
        />
        <FormField
          control={form.control}
          name="dataSource"
          render={({ field }) => (
            <FormItem className="space-y-2">
              <FormLabel>{copy.dataSourceLabel}</FormLabel>
              <FormControl>
                <Input placeholder="Datasource or catalogue" {...field} />
              </FormControl>
              <FormDescription>Source powering this widget</FormDescription>
            </FormItem>
          )}
        />
      </AccordionContent>
    </AccordionItem>
  );
}

/**
 * Display section inside the widget form accordion.
 * @param root0 - Section props.
 * @param root0.form - Hook form instance.
 * @param root0.copy - Copy translations.
 */
function DisplaySection({
  form,
  copy,
}: {
  readonly form: UseFormReturn<FormValues>;
  readonly copy: typeof templateScreenCopy.properties;
}) {
  return (
    <AccordionItem value="display" className="rounded-2xl border border-border/60">
      <AccordionTrigger>Display</AccordionTrigger>
      <AccordionContent className="space-y-4 pt-3">
        <FormField
          control={form.control}
          name="layout"
          render={({ field }) => (
            <FormItem className="space-y-2">
              <FormLabel>{copy.layoutLabel}</FormLabel>
              <FormControl>
                <Textarea placeholder="Layout hints or coordinates" rows={3} {...field} />
              </FormControl>
              <FormDescription>Grid position, size, or layout notes</FormDescription>
            </FormItem>
          )}
        />
        <div className="grid gap-3 sm:grid-cols-2">
          <FormField
            control={form.control}
            name="displayMode"
            render={({ field }) => (
              <FormItem className="space-y-2">
                <FormLabel>Display mode</FormLabel>
                <FormControl>
                  <Select onValueChange={field.onChange} value={field.value}>
                    <SelectTrigger>
                      <SelectValue placeholder="Default" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="default">Default</SelectItem>
                      <SelectItem value="compact">Compact</SelectItem>
                      <SelectItem value="expanded">Expanded</SelectItem>
                    </SelectContent>
                  </Select>
                </FormControl>
              </FormItem>
            )}
          />
          <FormField
            control={form.control}
            name="showLabels"
            render={({ field }) => (
              <FormItem className="flex flex-row items-center justify-between">
                <FormLabel className="text-sm">Show labels</FormLabel>
                <FormControl>
                  <Switch
                    checked={field.value}
                    onCheckedChange={(value) => {
                      field.onChange(value);
                    }}
                  />
                </FormControl>
              </FormItem>
            )}
          />
        </div>
      </AccordionContent>
    </AccordionItem>
  );
}

/**
 * Filter section inside the widget form accordion.
 * @param root0 - Section props.
 * @param root0.form - Hook form instance.
 */
function FiltersSection({ form }: { readonly form: UseFormReturn<FormValues> }) {
  return (
    <AccordionItem value="filters" className="rounded-2xl border border-border/60">
      <AccordionTrigger>Filters</AccordionTrigger>
      <AccordionContent className="space-y-4 pt-3">
        <FormField
          control={form.control}
          name="filter"
          render={({ field }) => (
            <FormItem className="space-y-2">
              <FormLabel>Filter expression</FormLabel>
              <FormControl>
                <Input placeholder="e.g. status == 'published'" {...field} />
              </FormControl>
              <FormDescription>Use the filter bar to focus on relevant data.</FormDescription>
            </FormItem>
          )}
        />
      </AccordionContent>
    </AccordionItem>
  );
}

/**
 * Interactions section inside the widget form accordion.
 * @param root0 - Section props.
 * @param root0.form - Hook form instance.
 */
function InteractionsSection({ form }: { readonly form: UseFormReturn<FormValues> }) {
  return (
    <AccordionItem value="interactions" className="rounded-2xl border border-border/60">
      <AccordionTrigger>Interactions</AccordionTrigger>
      <AccordionContent className="space-y-4 pt-3">
        <FormField
          control={form.control}
          name="interactionMode"
          render={({ field }) => (
            <FormItem className="space-y-2">
              <FormLabel>Interaction mode</FormLabel>
              <FormControl>
                <Select onValueChange={field.onChange} value={field.value}>
                  <SelectTrigger>
                    <SelectValue placeholder="Default" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="default">Default</SelectItem>
                    <SelectItem value="advanced">Advanced</SelectItem>
                  </SelectContent>
                </Select>
              </FormControl>
              <FormDescription>Controls how users can interact with this widget.</FormDescription>
            </FormItem>
          )}
        />
      </AccordionContent>
    </AccordionItem>
  );
}

/**
 * Renders the accordion-backed form for a single widget.
 * @param root0 - Component props.
 * @param root0.form - Hook form instance.
 * @param root0.copy - Copy map with localized labels.
 */
function WidgetFormPanel({ form, copy }: WidgetFormPanelProperties) {
  return (
    <Form {...form}>
      <Accordion type="single" collapsible defaultValue="data" className="space-y-3">
        <DataSection form={form} copy={copy} />
        <DisplaySection form={form} copy={copy} />
        <FiltersSection form={form} />
        <InteractionsSection form={form} />
      </Accordion>
    </Form>
  );
}

export interface PropertiesInspectorProperties {
  readonly selection: SelectionState;
  readonly selectionKind: SelectionKind;
  readonly selectionId?: string;
  readonly properties?: SelectionProperties;
  readonly onSave?: (patch: Record<string, string | undefined>) => void | Promise<void>;
  readonly onReset?: () => void;
  readonly saving?: boolean;
  readonly error?: string;
}

/**
 * Right-pane inspector for the active selection.
 * @param root0 - Component props.
 * @param root0.selection - Current selection state from the canvas.
 * @param root0.selectionKind - Derivation of the selection type.
 * @param root0.selectionId - Primary selection identifier.
 * @param root0.properties - Optional persisted properties for the selection.
 * @param root0.onSave - Callback invoked when the user saves changes.
 * @param root0.onReset - Callback invoked when the reset action is triggered.
 * @param root0.saving - Whether the inspector is busy saving changes.
 * @param root0.error - Optional error message to render.
 */
// eslint-disable-next-line sonarjs/cognitive-complexity
export function PropertiesInspector({
  selection,
  selectionKind,
  selectionId,
  properties,
  onSave,
  onReset,
  saving,
  error,
}: PropertiesInspectorProperties) {
  const copy = templateScreenCopy.properties;
  const selectionCount = selection.nodeIds.length + selection.edgeIds.length;
  const showWidgetForm = selectionKind === 'widget' && !!selectionId;
  const showMultiState = !showWidgetForm && selectionCount > 0;
  const showEmptyState = !showWidgetForm && !showMultiState;
  let badgeLabel = 'Page';
  if (showWidgetForm) {
    badgeLabel = 'Widget';
  } else if (showMultiState) {
    badgeLabel = 'Multi';
  }

  const headerDescription = useMemo(() => {
    if (showEmptyState) {
      return 'Select a widget to edit its data, display, or interactions.';
    }
    if (showMultiState) {
      return `${selectionCount.toString()} items selected. Use bulk actions to keep the storyboard tidy.`;
    }
    return copy.widgetHeading;
  }, [copy.widgetHeading, selectionCount, showEmptyState, showMultiState]);

  const form = useForm<FormValues>({
    defaultValues: {
      name: resolveFieldValue(properties?.name, selectionId),
      dataSource: properties?.dataSource ?? '',
      layout: properties?.layout ?? '',
      description: properties?.description ?? '',
      filter: '',
      displayMode: 'default',
      showLabels: true,
      interactionMode: 'default',
    },
  });

  useEffect(() => {
    if (!showWidgetForm) {
      return;
    }
    form.reset({
      name: resolveFieldValue(properties?.name, selectionId),
      dataSource: properties?.dataSource ?? '',
      layout: properties?.layout ?? '',
      description: properties?.description ?? '',
      filter: '',
      displayMode: 'default',
      showLabels: true,
      interactionMode: 'default',
    });
  }, [form, properties, selectionId, showWidgetForm]);

  const submit = form.handleSubmit(async (values) => {
    await onSave?.({
      name: values.name,
      dataSource: values.dataSource,
      layout: values.layout,
      description: values.description,
    });
  });

  const handleSave = () => {
    void submit().catch(() => {
      /* ignore */
    });
  };

  let footer: ReactNode | undefined;
  if (showWidgetForm) {
    footer = (
      <CardFooter className="flex flex-wrap items-center gap-4">
        <Button size="sm" onClick={handleSave} disabled={saving}>
          {saving ? 'Saving…' : 'Save changes'}
        </Button>
        <Button size="sm" variant="outline" onClick={onReset} disabled={saving}>
          Reset
        </Button>
        {error ? <p className="text-xs text-destructive">{error}</p> : undefined}
      </CardFooter>
    );
  } else if (showMultiState) {
    footer = (
      <CardFooter className="flex flex-wrap gap-4">
        <Button size="sm" variant="outline" onClick={alignBulkAction}>
          Align
        </Button>
        <Button size="sm" variant="outline" onClick={distributeBulkAction}>
          Distribute
        </Button>
        <Button size="sm" variant="destructive" onClick={deleteBulkAction}>
          Delete
        </Button>
      </CardFooter>
    );
  }

  return (
    <Card className="flex min-h-full flex-col border-border/60 bg-card/90 shadow-sm">
      <CardHeader className="space-y-2 p-4">
        <div className="flex items-center justify-between gap-2">
          <CardTitle>Properties</CardTitle>
          <Badge variant="secondary" className="text-[0.6rem] uppercase tracking-[0.36em]">
            {badgeLabel}
          </Badge>
        </div>
        <CardDescription>{headerDescription}</CardDescription>
      </CardHeader>

      <ScrollArea className="relative flex-1 overflow-hidden">
        <CardContent className="space-y-4 p-4">
          {showEmptyState && (
            <div className="space-y-4">
              <p className="text-sm text-muted-foreground">
                Select a widget to edit its data, display, or interactions.
              </p>
              <div className="rounded-2xl border border-border/60 bg-muted p-4 text-xs text-muted-foreground">
                <p className="font-semibold text-foreground">Page properties</p>
                <p>Mainline FY25 · Executive overview</p>
                <p>Last updated just now</p>
              </div>
            </div>
          )}

          {showMultiState && <MultiSelectionPanel selectionCount={selectionCount} />}

          {showWidgetForm && <WidgetFormPanel form={form} copy={copy} />}
        </CardContent>
      </ScrollArea>

      {footer}
    </Card>
  );
}
