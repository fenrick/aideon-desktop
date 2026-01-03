import { describe, expect, it } from 'vitest';

import type { GraphViewModel } from 'praxis/praxis-api';

import { buildFlowEdges, buildFlowNodes } from 'praxis/widgets/graph-transform';

const SAMPLE_VIEW: GraphViewModel = {
  metadata: {
    id: 'graph-1',
    name: 'Test graph',
    asOf: '2025-11-14T00:00:00.000Z',
    scenario: 'main',
    fetchedAt: '2025-11-14T00:00:00.000Z',
    source: 'mock',
  },
  stats: { nodes: 2, edges: 1 },
  nodes: [
    {
      id: 'node-1',
      label: 'Node A',
      type: 'Capability',
      position: { x: 10, y: 20 },
    },
    {
      id: 'node-2',
      label: 'Node B',
      type: 'Capability',
      position: { x: 30, y: 40 },
    },
  ],
  edges: [
    {
      id: undefined,
      from: 'node-1',
      to: 'node-2',
      type: 'depends_on',
      label: 'depends',
    },
  ],
};

describe('graph-transform', () => {
  it('creates Praxis node proxies with fallback positions and entity metadata', () => {
    const nodes = buildFlowNodes(SAMPLE_VIEW);
    expect(nodes).toHaveLength(2);
    expect(nodes[0]).toMatchObject({
      id: 'node-1',
      type: 'praxis-node',
      position: { x: 10, y: 20 },
      data: { entityTypes: ['Capability'], status: 'active' },
    });
    expect(nodes[1]).toMatchObject({
      id: 'node-2',
      position: { x: 30, y: 40 },
      data: { entityTypes: ['Capability'] },
    });
  });

  it('creates React Flow edges with stable identifiers and animation for dependency edges', () => {
    const edges = buildFlowEdges(SAMPLE_VIEW);
    expect(edges).toHaveLength(1);
    const [edge] = edges;
    expect(edge).toBeDefined();
    expect(edge).toMatchObject({
      source: 'node-1',
      target: 'node-2',
      animated: true,
      data: { label: 'depends' },
    });
    expect(edge?.id).toContain('node-1-node-2');
  });
});
