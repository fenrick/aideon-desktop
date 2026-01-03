import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import {
  getChartView,
  listTemporalCommits,
  mergeTemporalBranches,
  type ChartViewModel,
} from 'praxis/praxis-api';
import { describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
const invoke = vi.mocked(tauriInvoke);

vi.mock('praxis/platform', () => ({ isTauri: () => true }));

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

const baseMeta = {
  id: 'chart1',
  name: 'Chart',
  asOf: '2025-01-01T00:00:00Z',
  fetchedAt: '2025-01-01T00:00:00Z',
  source: 'host' as const,
};

describe('praxis-api host paths', () => {
  it('merges branches and surfaces conflicts', async () => {
    invoke.mockImplementationOnce(
      mockIpcOk({ conflicts: [{ reference: 'r1', kind: 'diverge', message: 'conflict' }] }),
    );

    const result = await mergeTemporalBranches({ source: 'a', target: 'b' });

    expect(result.conflicts?.[0]).toMatchObject({
      reference: 'r1',
      kind: 'diverge',
      message: 'conflict',
    });
  });

  it('fills in missing branch names when listing commits', async () => {
    invoke.mockImplementationOnce(
      mockIpcOk({ commits: [{ id: 'c1', parents: [], message: 'msg', change_count: 1 }] }),
    );

    const commits = await listTemporalCommits('dev');
    expect(commits[0]).toMatchObject({ id: 'c1', branch: 'dev', changeCount: 1 });
  });

  it('invokes host for chart view when in tauri', async () => {
    const chartView: ChartViewModel = {
      metadata: baseMeta,
      chartType: 'kpi',
      series: [],
    };
    invoke.mockImplementationOnce(mockIpcOk(chartView));

    const definition = {
      id: 'chart1',
      name: 'Chart',
      kind: 'chart',
      asOf: '2025-01-01T00:00:00Z',
      chartType: 'kpi',
      measure: 'm',
    } as const;

    await expect(getChartView(definition)).resolves.toMatchObject({
      chartType: 'kpi',
    });
    const calls = (invoke as unknown as { mock: { calls: unknown[][] } }).mock.calls;
    const invokeArguments = findInvokeArguments(calls, 'praxis.artefact.execute_chart');
    expect(payloadFromInvokeArguments(invokeArguments)).toEqual(definition);
  });
});
