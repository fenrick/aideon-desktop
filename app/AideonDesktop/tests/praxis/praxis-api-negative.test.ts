import { beforeEach, describe, expect, it, vi } from 'vitest';

beforeEach(() => {
  vi.resetModules();
});

/**
 * Narrow unknown values to plain object records.
 * @param value
 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
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

describe('praxis-api negative paths', () => {
  it('wraps host errors when Tauri invoke fails', async () => {
    const invokeMock = vi.fn().mockRejectedValue(new Error('boom'));
    vi.doMock('praxis/platform', () => ({ isTauri: () => true }));
    vi.doMock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));
    const { getGraphView } = await import('praxis/praxis-api');

    await expect(
      getGraphView({ id: 'g1', name: 'Graph', kind: 'graph', asOf: 'now' }),
    ).rejects.toThrow("Host command 'praxis.artefact.execute_graph' failed: boom");
  });

  it('normalises host branch/commit payloads from invoke responses', async () => {
    const invokeMock = vi
      .fn()
      // listBranches
      .mockImplementationOnce(
        mockIpcOk({ branches: [{ name: 'main' }, { head: 'abc' }] }),
      )
      // listCommits
      .mockImplementationOnce(
        mockIpcOk({
          commits: [
            {
              id: undefined,
              parents: ['p1'],
              tags: ['tag', 1],
              message: undefined,
              change_count: 'x',
            },
          ],
        }),
      );
    vi.doMock('praxis/platform', () => ({ isTauri: () => true }));
    vi.doMock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

    const { listTemporalBranches, listTemporalCommits } = await import('praxis/praxis-api');
    const branches = await listTemporalBranches();
    expect(branches).toEqual([
      { name: 'main', head: undefined },
      { name: '', head: 'abc' },
    ]);

    const commits = await listTemporalCommits('feat');
    expect(commits[0]).toMatchObject({
      id: 'unknown',
      branch: 'feat',
      message: 'Commit',
      tags: ['tag'],
      changeCount: 0,
    });
  });
});
