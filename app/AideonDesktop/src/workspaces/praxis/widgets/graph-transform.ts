import type { Edge, Node } from '@xyflow/react';

import type { TimelineEdgeData } from 'design-system/components/timeline-edge';
import type { GraphEdgeView, GraphNodeView, GraphViewModel } from 'praxis/praxis-api';
import type { GraphNodeData } from './graph-node-data';

/**
 * Convert a graph view model into React Flow nodes.
 * @param view domain view model from the Praxis API.
 * @returns array of typed nodes ready for XYFlow consumption.
 */
export function buildFlowNodes(view: GraphViewModel): Node<GraphNodeData>[] {
  return view.nodes.map((node) => toFlowNode(node));
}

/**
 * Convert a graph view model into React Flow edges.
 * @param view domain view model from the Praxis API.
 * @returns array of timeline edges for XYFlow.
 */
export function buildFlowEdges(view: GraphViewModel): Edge<TimelineEdgeData>[] {
  return view.edges.map((edge, index) => toFlowEdge(edge, index));
}

/**
 * Map a domain node into the praxis-node XYFlow shape.
 * @param node graph node view from the host.
 * @returns XYFlow node with typed metadata.
 */
export function toFlowNode(node: GraphNodeView): Node<GraphNodeData> {
  return {
    id: node.id,
    type: 'praxis-node',
    data: {
      label: node.label,
      typeLabel: node.type,
      status: node.type === 'Capability' ? 'active' : 'warning',
      meta: resolveDescription(node),
      entityTypes: resolveEntityTypes(node),
    },
    position: {
      x: node.position?.x ?? 0,
      y: node.position?.y ?? 0,
    },
  };
}

/**
 * Map a domain edge into the timeline edge type.
 * @param edge graph edge view from the host.
 * @param index fallback index for generating unique ids.
 * @returns XYFlow edge configured for the timeline renderer.
 */
export function toFlowEdge(edge: GraphEdgeView, index: number): Edge<TimelineEdgeData> {
  return {
    id: edge.id ?? `${edge.from}-${edge.to}-${String(index)}`,
    source: edge.from,
    target: edge.to,
    label: edge.label,
    type: 'timeline',
    data: {
      label: edge.label,
    },
    animated: edge.type === 'depends_on',
  };
}

/**
 * Extract a description from optional metadata if provided by the host.
 * @param node graph node view.
 * @returns description string when present; otherwise undefined.
 */
function resolveDescription(node: GraphNodeView): string | undefined {
  const metadata = (node as { metadata?: { description?: string } }).metadata;
  const description = metadata?.description;
  return typeof description === 'string' ? description : undefined;
}

/**
 * Derive the list of entity type labels for display.
 * @param node graph node view.
 * @returns array with at least one type; defaults to "Entity".
 */
function resolveEntityTypes(node: GraphNodeView): string[] {
  const typeValue = node.type;
  if (typeof typeValue === 'string' && typeValue.trim()) {
    return [typeValue];
  }
  return ['Entity'];
}
