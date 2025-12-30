import { useEffect, useMemo, useState } from 'react';

import { toErrorMessage } from 'praxis/lib/errors';
import { fetchMetaModel } from 'praxis/lib/meta-model';
import { searchStore } from 'praxis/lib/search';
import { getCatalogueView, type CatalogueRow, type TemporalCommitSummary } from 'praxis/praxis-api';
import { useTemporalPanel } from 'praxis/time/use-temporal-panel';

import { Button } from 'design-system/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from 'design-system/components/ui/card';
import {
  TemporalCommandMenu,
  type CatalogueCommandEntry,
  type MetaModelCommandEntry,
} from 'praxis/components/blocks/temporal-command-menu';

interface GlobalSearchCardProperties {
  readonly onSelectNodes?: (nodeIds: string[]) => void;
  readonly onFocusMetaModel?: (entry: MetaModelCommandEntry) => void;
  readonly onShowTimeline?: () => void;
}

/**
 *
 * @param root0
 * @param root0.onSelectNodes
 * @param root0.onFocusMetaModel
 * @param root0.onShowTimeline
 */
export function GlobalSearchCard({
  onSelectNodes,
  onFocusMetaModel,
  onShowTimeline,
}: GlobalSearchCardProperties = {}) {
  const [state, actions] = useTemporalPanel();
  const [commandOpen, setCommandOpen] = useState(false);
  const [catalogueEntries, setCatalogueEntries] = useState<CatalogueCommandEntry[]>([]);
  const [metaModelEntries, setMetaModelEntries] = useState<MetaModelCommandEntry[]>([]);
  const [commandStatus, setCommandStatus] = useState<string | undefined>();
  const [catalogueError, setCatalogueError] = useState<string | undefined>();

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        setCommandOpen(true);
      }
    };
    globalThis.addEventListener('keydown', handleKeyDown);
    return () => {
      globalThis.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    const loadAuxiliarySources = async () => {
      try {
        const [schema, catalogue] = await Promise.all([
          fetchMetaModel(),
          getCatalogueView({
            id: 'command-catalogue',
            name: 'Command palette quick search',
            kind: 'catalogue',
            asOf: new Date().toISOString(),
            scenario: state.branch ?? 'main',
            columns: [
              { id: 'name', label: 'Name', type: 'string' },
              { id: 'owner', label: 'Owner', type: 'string' },
              { id: 'state', label: 'State', type: 'string' },
            ],
            limit: 25,
          }),
        ]);
        if (cancelled) {
          return;
        }
        setMetaModelEntries(buildMetaModelEntries(schema));
        const newCatalogueEntries = buildCatalogueEntries(catalogue.rows);
        setCatalogueEntries(newCatalogueEntries);
        setCatalogueError(undefined);
        searchStore.setCatalogEntities(
          newCatalogueEntries.map((entry) => ({
            id: entry.id,
            name: entry.label,
            type: entry.state ?? 'Catalogue',
            description: entry.owner,
            sidebarId: 'catalogues',
          })),
          (entity) => {
            onSelectNodes?.([entity.id]);
          },
        );
      } catch (unknownError) {
        if (!cancelled) {
          setCatalogueError(toErrorMessage(unknownError));
        }
      }
    };
    void loadAuxiliarySources();
    return () => {
      cancelled = true;
    };
  }, [state.branch, onSelectNodes]);

  useEffect(() => {
    searchStore.setRecentCommits(state.commits, (commitId) => {
      onShowTimeline?.();
      actions.selectCommit(commitId);
    });
  }, [state.commits, actions, onShowTimeline]);

  const recentCommits = useMemo(() => {
    return state.commits.toSorted((left, right) => {
      const leftTime = left.time ? Date.parse(left.time) : 0;
      const rightTime = right.time ? Date.parse(right.time) : 0;
      return rightTime - leftTime;
    });
  }, [state.commits]);

  return (
    <>
      <Card>
        <CardHeader className="space-y-1">
          <CardTitle>Command palette</CardTitle>
          <CardDescription>
            Use ⌘K / Ctrl+K to switch timelines, jump to moments, or run actions.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4 text-sm">
          <div className="flex flex-wrap items-center gap-3">
            <Button
              size="sm"
              onClick={() => {
                setCommandOpen(true);
              }}
            >
              Open command palette
            </Button>
            <ShortcutHint keys={['⌘', 'K']} />
            <ShortcutHint keys={['Ctrl', 'K']} />
          </div>
          <p className="text-xs uppercase tracking-[0.3em] text-muted-foreground">Recent moments</p>
          {commandStatus ? (
            <p className="text-xs text-muted-foreground">Last command ·{commandStatus}</p>
          ) : undefined}
          {catalogueError ? (
            <p className="text-xs text-destructive">{catalogueError}</p>
          ) : undefined}
          {recentCommits.length === 0 ? (
            <p className="text-xs text-muted-foreground">
              No recent moments available. Start by creating a timeline.
            </p>
          ) : (
            <div className="space-y-2">
              {recentCommits.slice(0, 4).map((commit) => (
                <CommitPreview
                  key={commit.id}
                  commit={commit}
                  onSelectBranch={(branch) => {
                    void actions.selectBranch(branch);
                  }}
                  onSelectCommit={actions.selectCommit}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>
      <TemporalCommandMenu
        open={commandOpen}
        onOpenChange={setCommandOpen}
        branches={state.branches}
        activeBranch={state.branch}
        commits={state.commits}
        loading={state.loading}
        onSelectBranch={(branch) => {
          void actions.selectBranch(branch);
        }}
        onSelectCommit={(commitId) => {
          actions.selectCommit(commitId);
        }}
        onRefreshBranches={() => {
          void actions.refreshBranches();
        }}
        catalogueEntries={catalogueEntries}
        metaModelEntries={metaModelEntries}
        onSelectCatalogueEntry={(entry) => {
          setCommandStatus(`Catalogue · ${entry.label}`);
          onSelectNodes?.([entry.id]);
        }}
        onSelectMetaModelEntry={(entry) => {
          setCommandStatus(`Meta-model · ${entry.label}`);
          onFocusMetaModel?.(entry);
        }}
      />
    </>
  );
}

/**
 *
 * @param rows
 */
function buildCatalogueEntries(rows: CatalogueRow[]): CatalogueCommandEntry[] {
  return rows.map((row) => {
    const rawName = row.values.name;
    const rawOwner = row.values.owner;
    const rawState = row.values.state;
    return {
      id: row.id,
      label: typeof rawName === 'string' && rawName.trim() ? rawName : row.id,
      owner: typeof rawOwner === 'string' && rawOwner.trim() ? rawOwner : undefined,
      state: typeof rawState === 'string' && rawState.trim() ? rawState : undefined,
    } satisfies CatalogueCommandEntry;
  });
}

/**
 *
 * @param schema
 */
function buildMetaModelEntries(
  schema: Awaited<ReturnType<typeof fetchMetaModel>>,
): MetaModelCommandEntry[] {
  const typeEntries: MetaModelCommandEntry[] = schema.types.map((type) => ({
    id: type.id,
    label: type.label ?? type.id,
    category: type.category ?? 'Entity',
    kind: 'type',
  }));
  const relationshipEntries: MetaModelCommandEntry[] = schema.relationships.map((relationship) => ({
    id: relationship.id,
    label: relationship.label ?? relationship.id,
    category: `${relationship.from.join(', ')} → ${relationship.to.join(', ')}`,
    kind: 'relationship',
  }));
  return [...typeEntries, ...relationshipEntries];
}

/**
 *
 * @param root0
 * @param root0.keys
 */
function ShortcutHint({ keys }: { readonly keys: string[] }) {
  return (
    <div className="inline-flex items-center gap-1 text-xs text-muted-foreground">
      {keys.map((key) => (
        <kbd
          key={key}
          className="rounded border border-border/70 bg-muted/50 px-1 py-0.5 font-medium"
        >
          {key}
        </kbd>
      ))}
    </div>
  );
}

/**
 *
 * @param root0
 * @param root0.commit
 * @param root0.onSelectBranch
 * @param root0.onSelectCommit
 */
function CommitPreview({
  commit,
  onSelectBranch,
  onSelectCommit,
}: {
  readonly commit: TemporalCommitSummary;
  readonly onSelectBranch: (branch: string) => void;
  readonly onSelectCommit: (commitId?: string) => void;
}) {
  return (
    <div className="rounded-2xl border border-border/70 p-3">
      <p className="text-sm font-semibold">{commit.message}</p>
      <p className="text-xs text-muted-foreground">
        {commit.branch} ·{commit.tags.map((tag) => `#${tag}`).join(' ') || 'No tags'}
      </p>
      <div className="mt-2 flex gap-2">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            onSelectBranch(commit.branch);
          }}
        >
          Switch timeline
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            onSelectCommit(commit.id);
          }}
        >
          Jump to moment
        </Button>
      </div>
    </div>
  );
}
