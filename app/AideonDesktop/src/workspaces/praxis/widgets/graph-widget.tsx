import type { MouseEvent as ReactMouseEvent } from 'react';
import { useCallback, useEffect, useMemo, useState } from 'react';

import {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  Panel,
  ReactFlow,
  ReactFlowProvider,
  useEdgesState,
  useNodesState,
  type Edge,
  type EdgeTypes,
  type Node,
  type NodeTypes,
} from '@xyflow/react';
import { toErrorMessage } from 'praxis/lib/errors';
import { cn } from 'praxis/lib/utilities';
import { getGraphView, type GraphViewModel } from 'praxis/praxis-api';

import { NodeSearchDialog } from 'design-system/components/node-search';
import { PraxisNode } from 'design-system/components/praxis-node';
import { TimelineEdge, type TimelineEdgeData } from 'design-system/components/timeline-edge';
import { Button } from 'design-system/components/ui/button';
import { ToggleGroup, ToggleGroupItem } from 'design-system/components/ui/toggle-group';
import type {
  PraxisGraphWidgetConfig as GraphWidgetConfig,
  SelectionState,
  WidgetSelection,
} from 'praxis/types';
import type { GraphNodeData } from './graph-node-data';
import { areStringSetsEqual, selectionFromEvent } from './graph-selection';
import { buildFlowEdges, buildFlowNodes } from './graph-transform';
import { WidgetToolbar } from './widget-toolbar';

interface GraphWidgetProperties {
  readonly widget: GraphWidgetConfig;
  readonly reloadVersion: number;
  readonly selection?: SelectionState;
  readonly onSelectionChange?: (selection: WidgetSelection) => void;
  readonly onViewChange?: (view: GraphViewModel) => void;
  readonly onError?: (message: string) => void;
  readonly onRequestMetaModelFocus?: (types: string[]) => void;
}

/**
 *
 * @param root0
 * @param root0.widget
 * @param root0.reloadVersion
 * @param root0.selection
 * @param root0.onSelectionChange
 * @param root0.onViewChange
 * @param root0.onError
 * @param root0.onRequestMetaModelFocus
 */
export function GraphWidget({
  widget,
  reloadVersion,
  selection,
  onSelectionChange,
  onViewChange,
  onError,
  onRequestMetaModelFocus,
}: GraphWidgetProperties) {
  const [nodes, setNodes, handleNodesChange] = useNodesState<Node<GraphNodeData>>([]);
  const [edges, setEdges, handleEdgesChange] = useEdgesState<Edge<TimelineEdgeData>>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | undefined>();
  const [metadata, setMetadata] = useState<GraphViewModel['metadata'] | undefined>();
  const [background, setBackground] = useState<'dots' | 'lines' | 'cross'>('dots');
  const [showMiniMap, setShowMiniMap] = useState(true);
  const [showControls, setShowControls] = useState(true);

  const definition = useMemo(() => {
    return {
      ...widget.view,
      asOf: new Date().toISOString(),
    };
  }, [widget.view]);

  const attachInspectHandlers = useCallback(
    (flowNodes: Node<GraphNodeData>[]) => {
      if (!onRequestMetaModelFocus) {
        return flowNodes;
      }
      return flowNodes.map((node) => {
        const types = (node.data.entityTypes ?? []).filter((value: string) => {
          return isNonEmptyString(value);
        });
        if (types.length === 0) {
          return node;
        }
        return {
          ...node,
          data: {
            ...node.data,
            onInspect: () => {
              onRequestMetaModelFocus(types);
            },
          },
        };
      });
    },
    [onRequestMetaModelFocus],
  );

  const loadView = useCallback(async () => {
    setLoading(true);
    setError(undefined);
    try {
      const view = await getGraphView(definition);
      setMetadata(view.metadata);
      const flowNodes = buildFlowNodes(view);
      setNodes(attachInspectHandlers(flowNodes));
      setEdges(buildFlowEdges(view));
      onViewChange?.(view);
    } catch (unknownError) {
      const message = toErrorMessage(unknownError);
      setError(message);
      onError?.(message);
    } finally {
      setLoading(false);
    }
  }, [attachInspectHandlers, definition, onError, onViewChange, setEdges, setNodes]);

  useEffect(() => {
    loadView().catch((_ignoredError: unknown) => {
      return;
    });
  }, [loadView, reloadVersion]);

  useEffect(() => {
    setNodes((current) => attachInspectHandlers(current));
  }, [attachInspectHandlers, setNodes]);

  useEffect(() => {
    const selectedNodeIds = selection?.nodeIds ?? [];
    const selectedEdgeIds = selection?.edgeIds ?? [];
    const selectedNodeIdSet = new Set(selectedNodeIds);
    const selectedEdgeIdSet = new Set(selectedEdgeIds);

    setNodes((current) => {
      let didChange = false;
      const next: typeof current = [];
      for (const node of current) {
        const isSelected = selectedNodeIdSet.has(node.id);
        if (node.selected === isSelected) {
          next.push(node);
        } else {
          didChange = true;
          next.push({ ...node, selected: isSelected });
        }
      }
      return didChange ? next : current;
    });

    setEdges((current) => {
      let didChange = false;
      const next: typeof current = [];
      for (const edge of current) {
        const isSelected = selectedEdgeIdSet.has(edge.id);
        if (edge.selected === isSelected) {
          next.push(edge);
        } else {
          didChange = true;
          next.push({ ...edge, selected: isSelected });
        }
      }
      return didChange ? next : current;
    });
  }, [selection, setEdges, setNodes]);

  const handleSelection = useCallback(
    (nextSelection: { nodes?: Node[]; edges?: Edge[] }) => {
      const snapshot = selectionFromEvent(nextSelection);
      if (
        selection &&
        areStringSetsEqual(selection.nodeIds, snapshot.nodeIds) &&
        areStringSetsEqual(selection.edgeIds, snapshot.edgeIds)
      ) {
        return;
      }

      onSelectionChange?.({
        widgetId: widget.id,
        nodeIds: [...snapshot.nodeIds],
        edgeIds: [...snapshot.edgeIds],
      });
    },
    [onSelectionChange, selection, widget.id],
  );

  const [nodeSearchOpen, setNodeSearchOpen] = useState(false);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | undefined>();
  const nodeTypes = useMemo<NodeTypes>(
    () => ({ 'praxis-node': PraxisNode as NodeTypes['praxis-node'] }),
    [],
  );
  const edgeTypes = useMemo<EdgeTypes>(
    () => ({ timeline: TimelineEdge as EdgeTypes['timeline'] }),
    [],
  );

  const handleNodeContextMenu = useCallback(
    (event: ReactMouseEvent, node: Node<GraphNodeData>) => {
      event.preventDefault();
      const selectedNodes = nodes.filter((entry) => entry.selected);
      if (selectedNodes.length === 0 || !node.selected) {
        return;
      }
      const types = [...new Set(selectedNodes.map((entry) => resolveNodeType(entry)))];
      setContextMenu({
        x: event.clientX,
        y: event.clientY,
        types: types.length > 0 ? types : ['Entity'],
      });
    },
    [nodes],
  );

  useEffect(() => {
    if (!contextMenu) {
      return;
    }
    const dismiss = () => {
      setContextMenu(undefined);
    };
    document.addEventListener('click', dismiss);
    document.addEventListener('contextmenu', dismiss);
    return () => {
      document.removeEventListener('click', dismiss);
      document.removeEventListener('contextmenu', dismiss);
    };
  }, [contextMenu]);

  return (
    <div className="relative h-full w-full">
      <WidgetToolbar
        metadata={metadata}
        fallbackTitle={widget.title}
        loading={loading}
        onRefresh={() => {
          loadView().catch((_ignoredError: unknown) => {
            return;
          });
        }}
      />
      <div className="h-full min-h-[360px] w-full rounded-2xl border border-border/60 bg-muted/20">
        <ReactFlowProvider>
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={handleNodesChange}
            onEdgesChange={handleEdgesChange}
            fitView
            nodeTypes={nodeTypes}
            edgeTypes={edgeTypes}
            onSelectionChange={handleSelection}
            onNodeContextMenu={handleNodeContextMenu}
            onPaneClick={() => {
              setContextMenu(undefined);
            }}
            onPaneContextMenu={() => {
              setContextMenu(undefined);
            }}
          >
            <Background
              color="hsl(var(--muted-foreground))"
              variant={resolveBackgroundVariant(background)}
              gap={16}
              size={1}
            />
            {showControls ? <Controls position="bottom-right" /> : undefined}
            {showMiniMap ? (
              <MiniMap
                position="top-right"
                nodeColor={() => 'hsl(var(--primary) / 0.85)'}
                maskColor="hsl(var(--background) / 0.85)"
                className="rounded-xl border border-border/60 bg-background/80 shadow-sm"
              />
            ) : undefined}
            <Panel
              position="top-left"
              className="rounded-2xl border border-border/60 bg-background/90 p-3 text-xs text-muted-foreground shadow"
            >
              <div className="flex flex-wrap items-center gap-2">
                <ToggleGroup
                  type="single"
                  value={background}
                  onValueChange={(value) => {
                    if (value === 'dots' || value === 'lines' || value === 'cross') {
                      setBackground(value);
                    }
                  }}
                  className="gap-1"
                >
                  <ToggleGroupItem value="dots" aria-label="Dots background" className="h-7 px-2">
                    Dots
                  </ToggleGroupItem>
                  <ToggleGroupItem value="lines" aria-label="Lines background" className="h-7 px-2">
                    Lines
                  </ToggleGroupItem>
                  <ToggleGroupItem value="cross" aria-label="Cross background" className="h-7 px-2">
                    Cross
                  </ToggleGroupItem>
                </ToggleGroup>
                <Button
                  variant={showMiniMap ? 'secondary' : 'ghost'}
                  size="sm"
                  className="h-7 px-2 text-xs"
                  onClick={() => {
                    setShowMiniMap((previous) => !previous);
                  }}
                >
                  Mini map
                </Button>
                <Button
                  variant={showControls ? 'secondary' : 'ghost'}
                  size="sm"
                  className="h-7 px-2 text-xs"
                  onClick={() => {
                    setShowControls((previous) => !previous);
                  }}
                >
                  Controls
                </Button>
              </div>
              <p className="mt-2 text-[11px] text-muted-foreground/90">
                Use node search or right-click selection for meta actions.
              </p>
              <Button
                variant="ghost"
                size="sm"
                className="mt-1 px-0 text-xs"
                onClick={() => {
                  setNodeSearchOpen(true);
                }}
              >
                Open node search
              </Button>
            </Panel>
          </ReactFlow>
        </ReactFlowProvider>
        {loading ? <GraphWidgetOverlay message="Loading graph" /> : undefined}
        {error ? <GraphWidgetOverlay isError message={error} /> : undefined}
        {contextMenu ? (
          <GraphContextMenu
            x={contextMenu.x}
            y={contextMenu.y}
            onFocus={() => {
              onRequestMetaModelFocus?.(contextMenu.types);
              setContextMenu(undefined);
            }}
          />
        ) : undefined}
      </div>
      <NodeSearchDialog
        open={nodeSearchOpen}
        onOpenChange={setNodeSearchOpen}
        onSelectNode={(node: Node) => {
          handleSelection({ nodes: [node], edges: [] });
          setNodeSearchOpen(false);
        }}
      />
    </div>
  );
}

interface ContextMenuState {
  readonly x: number;
  readonly y: number;
  readonly types: string[];
}

/**
 *
 * @param node
 */
function resolveNodeType(node: Node<GraphNodeData>): string {
  const typeValue = node.data.typeLabel;
  if (typeof typeValue === 'string' && typeValue.trim()) {
    return typeValue;
  }
  const fallback = (node.data.entityTypes ?? []).find((value: string) => isNonEmptyString(value));
  return fallback ?? 'Entity';
}

interface GraphWidgetOverlayProperties {
  readonly message: string;
  readonly isError?: boolean;
}

/**
 * Map a background selection to the XYFlow variant enum.
 * @param background - Selected background type.
 * @returns Background variant.
 */
function resolveBackgroundVariant(background: 'dots' | 'lines' | 'cross'): BackgroundVariant {
  switch (background) {
    case 'dots': {
      return BackgroundVariant.Dots;
    }
    case 'lines': {
      return BackgroundVariant.Lines;
    }
    case 'cross': {
      return BackgroundVariant.Cross;
    }
  }
}

/**
 *
 * @param root0
 * @param root0.message
 * @param root0.isError
 */
function GraphWidgetOverlay({ message, isError }: GraphWidgetOverlayProperties) {
  return (
    <div
      className={cn(
        'absolute inset-0 flex items-center justify-center rounded-2xl text-sm backdrop-blur',
        isError ? 'bg-destructive/10 text-destructive' : 'bg-background/70 text-muted-foreground',
      )}
    >
      {isError ? <AlertBadge /> : undefined}
      {message}
    </div>
  );
}

/**
 *
 */
function AlertBadge() {
  return (
    <span className="mr-2 text-xs font-semibold uppercase tracking-wide text-destructive">
      Error
    </span>
  );
}

/**
 *
 * @param root0
 * @param root0.x
 * @param root0.y
 * @param root0.onFocus
 */
function GraphContextMenu({
  x,
  y,
  onFocus,
}: {
  readonly x: number;
  readonly y: number;
  readonly onFocus: () => void;
}) {
  return (
    <div
      className="fixed z-50 min-w-[160px] rounded-lg border border-border/70 bg-popover text-sm shadow-xl"
      style={{ top: y, left: x }}
    >
      <button
        type="button"
        className="block w-full px-4 py-2 text-left hover:bg-muted"
        onClick={onFocus}
      >
        View meta-model entry
      </button>
    </div>
  );
}

/**
 *
 * @param value
 */
function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.trim().length > 0;
}
