import { describe, expect, it, vi } from 'vitest';

vi.mock('praxis/platform', () => ({ isTauri: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

import { invoke } from '@tauri-apps/api/core';
import { isTauri } from 'praxis/platform';

import {
  applyOperations,
  getCatalogueView,
  getChartView,
  getGraphView,
  getMatrixView,
  getStateAtSnapshot,
  listTemporalCommits,
  mergeTemporalBranches,
} from 'praxis/praxis-api';

const isTauriMock = vi.mocked(isTauri);
const invokeMock = vi.mocked(invoke);

/**
 * Narrow unknown values to plain object records.
 * @param value
 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

/**
 * Extract payload from invoke arguments.
 * @param invokeArguments
 */
function payloadFromInvokeArguments(invokeArguments: unknown): unknown {
  if (!isRecord(invokeArguments)) {
    return undefined;
  }
  const request = invokeArguments.request;
  if (!isRecord(request)) {
    return undefined;
  }
  return request.payload;
}

/**
 * Find invoke args for a given command.
 * @param calls
 * @param command
 */
function findInvokeArguments(calls: unknown[][], command: string): unknown {
  const call = calls.find((entry) => entry[0] === command);
  return call?.[1];
}

/**
 * Create a successful IPC envelope response for the adapter boundary.
 * @param result
 */
function mockIpcOk(result: unknown) {
  return (_command: string, invokeArguments: unknown) => {
    const request = isRecord(invokeArguments) ? invokeArguments.request : undefined;
    const requestId =
      isRecord(request) && typeof request.requestId === 'string' ? request.requestId : 'req';
    return Promise.resolve({ requestId, status: 'ok', result });
  };
}

describe('praxis-api mock branches', () => {
  it('covers mock builders for catalogue, matrix, chart, commits, merge, operations, state-at', async () => {
    isTauriMock.mockReturnValue(false);

    const catalogue = await getCatalogueView({
      id: 'cat',
      name: 'Catalogue',
      kind: 'catalogue',
      asOf: '2025-01-01T00:00:00Z',
      columns: [],
    });
    expect(catalogue.columns.length).toBeGreaterThan(0);

    const matrix = await getMatrixView({
      id: 'm1',
      name: 'Matrix',
      kind: 'matrix',
      asOf: '2025-01-01T00:00:00Z',
      rowType: 'Capability',
      columnType: 'Service',
      relationship: 'depends_on',
    });
    expect(matrix.cells.length).toBeGreaterThan(0);

    const kpi = await getChartView({
      id: 'c1',
      name: 'KPI',
      kind: 'chart',
      asOf: '2025-01-01T00:00:00Z',
      chartType: 'kpi',
      measure: 'count',
    });
    expect(kpi.kpi?.value).toBeGreaterThan(0);

    const line = await getChartView({
      id: 'c2',
      name: 'Line',
      kind: 'chart',
      asOf: '2025-01-01T00:00:00Z',
      chartType: 'line',
      measure: 'velocity',
    });
    expect(line.series[0]?.points.length).toBe(7);

    const bar = await getChartView({
      id: 'c3',
      name: 'Bar',
      kind: 'chart',
      asOf: '2025-01-01T00:00:00Z',
      chartType: 'bar',
      measure: 'score',
    });
    expect(bar.series.length).toBe(2);

    const commits = await listTemporalCommits('chronaplay');
    expect(commits.length).toBeGreaterThan(0);
    expect(commits[0]?.branch).toBe('chronaplay');

    const merge = await mergeTemporalBranches({ source: 'chronaplay', target: 'main' });
    expect(merge.result).toBe('conflicts');
    expect(merge.conflicts?.[0]?.reference).toBeTruthy();

    const opResult = await applyOperations([{ kind: 'deleteNode', nodeId: 'n1' }]);
    expect(opResult.accepted).toBe(true);
    expect(opResult.commitId).toMatch(/^mock-commit-/);

    const state = await getStateAtSnapshot({ asOf: '2025-01-01T00:00:00Z' });
    expect(state.scenario).toBeDefined();
    expect(typeof state.nodes).toBe('number');
  });

  it('invokes host on success when in Tauri', async () => {
    isTauriMock.mockReturnValue(true);
    const graphView = {
      metadata: {
        id: 'g1',
        name: 'Graph',
        asOf: '2025-01-01',
        fetchedAt: '2025-01-01',
        source: 'host' as const,
      },
      stats: { nodes: 1, edges: 0 },
      nodes: [{ id: 'n1', label: 'Node 1' }],
      edges: [],
    };
    invokeMock.mockImplementationOnce(mockIpcOk(graphView));

    await expect(
      getGraphView({ id: 'g1', name: 'Graph', kind: 'graph', asOf: '2025-01-01' }),
    ).resolves.toMatchObject({ stats: { nodes: 1 } });
    const calls = (invokeMock as unknown as { mock: { calls: unknown[][] } }).mock.calls;
    const invokeArguments = findInvokeArguments(calls, 'praxis.artefact.execute_graph');
    expect(payloadFromInvokeArguments(invokeArguments)).toEqual({
      id: 'g1',
      name: 'Graph',
      kind: 'graph',
      asOf: '2025-01-01',
    });
  });
});
