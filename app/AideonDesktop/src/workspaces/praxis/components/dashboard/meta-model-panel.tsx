import { useCallback, useEffect, useMemo, useState } from 'react';

import { fetchMetaModel } from 'praxis/lib/meta-model';

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
  CardHeader,
  CardTitle,
} from 'design-system/components/ui/card';

type Status = 'idle' | 'loading' | 'ready' | 'error';
type MetaModelSchema = Awaited<ReturnType<typeof fetchMetaModel>>;

interface MetaModelPanelProperties {
  readonly focusEntryId?: string;
}

/**
 *
 * @param root0
 * @param root0.focusEntryId
 */
export function MetaModelPanel({ focusEntryId }: MetaModelPanelProperties = {}) {
  const [status, setStatus] = useState<Status>('idle');
  const [schema, setSchema] = useState<MetaModelSchema | undefined>();
  const [error, setError] = useState<string | undefined>();

  const loadSchema = useCallback(async () => {
    setStatus('loading');
    setError(undefined);
    try {
      const result = await fetchMetaModel();
      setSchema(result);
      setStatus('ready');
    } catch (unknownError) {
      const message = unknownError instanceof Error ? unknownError.message : String(unknownError);
      setError(message);
      setStatus('error');
    }
  }, []);

  useEffect(() => {
    queueMicrotask(() => {
      loadSchema().catch((_ignoredError: unknown) => {
        return;
      });
    });
  }, [loadSchema]);

  return (
    <Card>
      <CardHeader className="space-y-1">
        <CardTitle>Meta-model inspector</CardTitle>
        <CardDescription>
          {schema?.description ?? 'Schema reference for types and relationships.'}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
          <Badge variant="secondary">Version ·{schema?.version ?? 'loading'}</Badge>
          <Badge variant="outline">{status === 'ready' ? 'Live' : status}</Badge>
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              loadSchema().catch((_ignoredError: unknown) => {
                return;
              });
            }}
            disabled={status === 'loading'}
          >
            Reload schema
          </Button>
        </div>
        {renderSchemaState({
          status,
          schema,
          error,
          focusEntryId,
          onRetry: () => {
            loadSchema().catch((_ignoredError: unknown) => {
              return;
            });
          },
        })}
      </CardContent>
    </Card>
  );
}

const renderSchemaState = ({
  status,
  schema,
  error,
  focusEntryId,
  onRetry,
}: {
  readonly status: Status;
  readonly schema: MetaModelSchema | undefined;
  readonly error: string | undefined;
  readonly focusEntryId?: string;
  readonly onRetry: () => void;
}) => {
  if (status === 'error') {
    return (
      <div className="rounded-2xl border border-destructive/40 bg-destructive/5 p-4 text-xs">
        <p className="font-semibold text-destructive">Failed to load meta-model</p>
        <p className="text-destructive/80">{error}</p>
        <Button variant="outline" size="sm" className="mt-3" onClick={onRetry}>
          Retry load
        </Button>
      </div>
    );
  }
  if (status === 'loading' && !schema) {
    return <p className="text-xs text-muted-foreground">Fetching schema…</p>;
  }
  if (!schema) {
    return <p className="text-xs text-muted-foreground">Schema not available yet.</p>;
  }
  return <SchemaDetails schema={schema} focusEntryId={focusEntryId} />;
};

/**
 *
 * @param root0
 * @param root0.schema
 * @param root0.focusEntryId
 */
function SchemaDetails({
  schema,
  focusEntryId,
}: {
  readonly schema: MetaModelSchema;
  readonly focusEntryId?: string;
}) {
  useEffect(() => {
    if (!focusEntryId) {
      return;
    }
    const element = document.querySelector<HTMLElement>(`[data-metamodel-entry="${focusEntryId}"]`);
    element?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }, [focusEntryId, schema]);

  const focusedIds = useMemo(
    () => new Set([focusEntryId].filter(Boolean) as string[]),
    [focusEntryId],
  );

  return (
    <div className="space-y-6">
      <div className="grid gap-4 rounded-2xl border border-border/60 bg-muted/10 p-4 text-center text-sm">
        <StatBlock label="Types" value={schema.types.length} />
        <StatBlock label="Relationships" value={schema.relationships.length} />
      </div>
      <div className="grid gap-4 lg:grid-cols-2">
        <Accordion defaultValue="types" type="single" collapsible>
          <AccordionItem value="types">
            <AccordionTrigger className="text-sm font-semibold">Types</AccordionTrigger>
            <AccordionContent>
              <div className="flex flex-col gap-3">
                {schema.types.map((type) => (
                  <MetaModelEntryCard
                    key={type.id}
                    id={type.id}
                    label={type.label ?? type.id}
                    description={type.category ?? 'Type'}
                    extra={type.extends ? `extends ${type.extends}` : undefined}
                    focused={focusedIds.has(type.id)}
                  >
                    <AttributesList attributes={type.attributes} />
                  </MetaModelEntryCard>
                ))}
              </div>
            </AccordionContent>
          </AccordionItem>
        </Accordion>
        <Accordion defaultValue="relationships" type="single" collapsible>
          <AccordionItem value="relationships">
            <AccordionTrigger className="text-sm font-semibold">Relationships</AccordionTrigger>
            <AccordionContent>
              <div className="flex flex-col gap-3">
                {schema.relationships.map((relationship) => (
                  <MetaModelEntryCard
                    key={relationship.id}
                    id={relationship.id}
                    label={relationship.label ?? relationship.id}
                    description={`${relationship.from.join(', ')} → ${relationship.to.join(', ')}`}
                    extra={relationship.directed === false ? 'Undirected' : 'Directed'}
                    focused={focusedIds.has(relationship.id)}
                  >
                    <AttributesList attributes={relationship.attributes} />
                  </MetaModelEntryCard>
                ))}
              </div>
            </AccordionContent>
          </AccordionItem>
        </Accordion>
      </div>
    </div>
  );
}

/**
 *
 * @param root0
 * @param root0.label
 * @param root0.value
 */
function StatBlock({ label, value }: { readonly label: string; readonly value: number }) {
  return (
    <div>
      <p className="text-xs uppercase tracking-[0.3em] text-muted-foreground">{label}</p>
      <p className="text-2xl font-semibold">{value}</p>
    </div>
  );
}

/**
 *
 * @param root0
 * @param root0.id
 * @param root0.label
 * @param root0.description
 * @param root0.extra
 * @param root0.focused
 * @param root0.children
 */
function MetaModelEntryCard({
  id,
  label,
  description,
  extra,
  focused,
  children,
}: {
  readonly id: string;
  readonly label: string;
  readonly description: string;
  readonly extra?: string;
  readonly focused: boolean;
  readonly children: React.ReactNode;
}) {
  return (
    <article
      data-metamodel-entry={id}
      className={`rounded-2xl border p-4 transition ${
        focused ? 'border-primary/60 bg-primary/5 shadow-inner' : 'border-border/60 bg-card'
      }`}
    >
      <div className="flex items-center justify-between gap-3">
        <div>
          <p className="text-sm font-semibold">{label}</p>
          <p className="text-xs text-muted-foreground">{description}</p>
        </div>
        {extra && (
          <Badge variant="secondary" className="text-[0.6rem] uppercase tracking-[0.3em]">
            {extra}
          </Badge>
        )}
      </div>
      {children}
    </article>
  );
}

/**
 *
 * @param root0
 * @param root0.attributes
 */
function AttributesList({
  attributes,
}: {
  readonly attributes?: MetaModelSchema['types'][number]['attributes'];
}) {
  if (!attributes || attributes.length === 0) {
    return <p className="pt-2 text-xs text-muted-foreground">No attributes defined.</p>;
  }
  return (
    <dl className="mt-2 space-y-2 text-xs">
      {attributes.map((attribute) => (
        <div key={attribute.name} className="flex justify-between gap-2">
          <dt className="font-medium">
            {attribute.name}
            {attribute.required && (
              <span className="ml-2 rounded-full bg-destructive/10 px-2 py-0.5 text-[0.65rem] text-destructive">
                required
              </span>
            )}
          </dt>
          <dd className="text-muted-foreground">
            {attribute.type}
            {attribute.enum?.length ? ` [${attribute.enum.join(', ')}]` : ''}
          </dd>
        </div>
      ))}
    </dl>
  );
}
