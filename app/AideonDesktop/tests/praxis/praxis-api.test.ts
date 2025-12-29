import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('praxis/platform', () => ({ isTauri: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

import { invoke } from '@tauri-apps/api/core';
import { isTauri } from 'praxis/platform';

import {
  applyOperations,
  getGraphView,
  getTemporalDiff,
  getWorkerHealth,
  listScenarios,
  listTemporalBranches,
  listTemporalCommits,
  mergeTemporalBranches,
  type TemporalMergeConflict,
  type TemporalMergeResult,
} from 'praxis/praxis-api';

const isTauriMock = vi.mocked(isTauri);
const invokeMock = vi.mocked(invoke);

describe('praxis-api fallbacks and normalization', () => {
  beforeEach(() => {
    isTauriMock.mockReturnValue(false);
    invokeMock.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns mock worker health outside Tauri with fresh timestamp', async () => {
    vi.useFakeTimers();
    const now = new Date('2025-01-02T03:04:05Z');
    vi.setSystemTime(now);

    const snapshot = await getWorkerHealth();

    expect(snapshot.status).toBe('mock');
    expect(snapshot.ok).toBe(true);
    expect(snapshot.timestamp_ms).toBe(now.getTime());
  });

  it('normalizes branch payloads from the host', async () => {
    isTauriMock.mockReturnValue(true);
    invokeMock.mockResolvedValue({ branches: [{ name: 'main', head: undefined }, { head: 'h1' }] });

    const branches = await listTemporalBranches();

    expect(branches).toEqual([
      { name: 'main', head: undefined },
      { name: '', head: 'h1' },
    ]);
  });

  it('normalizes commit payloads and falls back to branch name', async () => {
    isTauriMock.mockReturnValue(true);
    invokeMock.mockResolvedValue({
      commits: [{ id: 'c1', parents: ['p1', 42], tags: ['t1', 99], change_count: 3 }, {}],
    });

    const commits = await listTemporalCommits('main');

    expect(commits[0]).toMatchObject({
      id: 'c1',
      branch: 'main',
      parents: ['p1'],
      tags: ['t1'],
      changeCount: 3,
    });
    expect(commits[1]).toMatchObject({ id: 'unknown', branch: 'main', message: 'Commit' });
  });

  it('wraps host errors when invoking commands inside Tauri', async () => {
    isTauriMock.mockReturnValue(true);
    invokeMock.mockRejectedValue(new Error('bad news'));

    await expect(
      getGraphView({
        id: 'g1',
        name: 'Graph',
        kind: 'graph',
        asOf: '2025-01-01',
      }),
    ).rejects.toThrow("Host command 'praxis_graph_view' failed: bad news");
  });

  it('returns deterministic mock diff summary when offline', async () => {
    const summary = await getTemporalDiff({ from: 'a', to: 'b', scope: 'cap' });

    expect(summary.from).toBe('a');
    expect(summary.to).toBe('b');
    expect(summary.metrics).toMatchObject({ nodeAdds: 3, edgeAdds: 4 });
  });

  it('drops malformed merge conflicts and infers a conflicts result', async () => {
    isTauriMock.mockReturnValue(true);
    invokeMock.mockResolvedValue({
      conflicts: [{ reference: 'cap-1', kind: 5 }, { kind: 'node' }],
    });

    const result: TemporalMergeResult = await mergeTemporalBranches({
      source: 'chronaplay',
      target: 'main',
    });

    expect(result.result).toBe('conflicts');
    const rawConflicts: unknown[] = Array.isArray(result.conflicts) ? result.conflicts : [];
    const conflicts: TemporalMergeConflict[] = rawConflicts.filter(
      (conflict): conflict is TemporalMergeConflict =>
        typeof (conflict as { reference?: unknown }).reference === 'string' &&
        typeof (conflict as { kind?: unknown }).kind === 'string' &&
        typeof (conflict as { message?: unknown }).message === 'string',
    );
    expect(conflicts).toHaveLength(1);
    expect(conflicts).not.toHaveLength(0);
    expect(conflicts[0]!).toMatchObject({ reference: 'cap-1', kind: 'unknown' });
    expect(typeof conflicts[0]!.message).toBe('string');
  });

  it('rejects empty operation batches in mock mode', async () => {
    const response = await applyOperations([]);

    expect(response.accepted).toBe(false);
    expect(response.message).toMatch(/No operations/);
  });

  it('uses mock scenarios outside Tauri', async () => {
    const scenarios = await listScenarios();

    expect(scenarios.length).toBeGreaterThan(0);
    expect(scenarios.some((scenario) => scenario.isDefault)).toBe(true);
  });
});
