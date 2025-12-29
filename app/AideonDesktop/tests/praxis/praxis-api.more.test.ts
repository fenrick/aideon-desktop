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

const baseMeta = {
  id: 'chart1',
  name: 'Chart',
  asOf: '2025-01-01T00:00:00Z',
  fetchedAt: '2025-01-01T00:00:00Z',
  source: 'host' as const,
};

describe('praxis-api host paths', () => {
  it('merges branches and surfaces conflicts', async () => {
    invoke.mockResolvedValueOnce({
      conflicts: [{ reference: 'r1', kind: 'diverge', message: 'conflict' }],
    });

    const result = await mergeTemporalBranches({ source: 'a', target: 'b' });

    expect(result.conflicts?.[0]).toMatchObject({
      reference: 'r1',
      kind: 'diverge',
      message: 'conflict',
    });
  });

  it('fills in missing branch names when listing commits', async () => {
    invoke.mockResolvedValueOnce({
      commits: [{ id: 'c1', parents: [], message: 'msg', change_count: 1 }],
    });

    const commits = await listTemporalCommits('dev');
    expect(commits[0]).toMatchObject({ id: 'c1', branch: 'dev', changeCount: 1 });
  });

  it('invokes host for chart view when in tauri', async () => {
    const chartView: ChartViewModel = {
      metadata: baseMeta,
      chartType: 'kpi',
      series: [],
    };
    invoke.mockResolvedValueOnce(chartView);

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
    const definitionMatcher = expect.objectContaining({ kind: 'chart' }) as unknown;
    expect(invoke).toHaveBeenCalledWith('praxis_chart_view', { definition: definitionMatcher });
  });
});
