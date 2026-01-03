import { beforeEach, describe, expect, it, vi } from 'vitest';

const callLog: { cmd: string; args?: Record<string, unknown> }[] = [];
const overrides = new Map<string, unknown>();

/**
 * Wrap a raw result into the IPC response envelope expected by `invokeIpc`.
 * @param result - Result payload.
 * @param arguments_ - Invoke arguments containing the request envelope.
 */
function ok(result: unknown, arguments_: Record<string, unknown> | undefined) {
  const requestId = (arguments_ as { request: { requestId: string } }).request.requestId;
  return Promise.resolve({ requestId, status: 'ok', result });
}

/**
 * Extract a commit reference ID (e.g. `{ id: "c1" }`) from an unknown value.
 * @param value - Potential commit reference.
 */
function commitReferenceId(value: unknown): string | undefined {
  if (typeof value !== 'object' || value === null) {
    return undefined;
  }
  const record = value as Record<string, unknown>;
  return typeof record.id === 'string' ? record.id : undefined;
}

/**
 * Narrow unknown values to plain object records.
 * @param value
 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

/**
 * Extract the request payload from a logged invoke call.
 * @param entry - Logged invoke call.
 */
function callPayload(
  entry: { cmd: string; args?: Record<string, unknown> } | undefined,
): Record<string, unknown> | undefined {
  const request = entry?.args?.request;
  if (!isRecord(request)) {
    return undefined;
  }
  const payload = request.payload;
  if (!isRecord(payload)) {
    return undefined;
  }
  return payload;
}

/**
 * Extract the requestId from a logged invoke call.
 * @param entry - Logged invoke call.
 */
function callRequestId(entry: { cmd: string; args?: Record<string, unknown> } | undefined) {
  const request = entry?.args?.request;
  if (!isRecord(request)) {
    return;
  }
  const requestId = request.requestId;
  if (typeof requestId !== 'string') {
    return;
  }
  return requestId;
}

/**
 * Require a command to have been invoked in the call log.
 * @param command
 */
function requireCall(command: string) {
  const entry = callLog.find((logged) => logged.cmd === command);
  if (!entry) {
    throw new Error(`Expected IPC call '${command}'.`);
  }
  return entry;
}

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, arguments_?: Record<string, unknown>) => {
    callLog.push({ cmd, args: arguments_ });
    if (overrides.has(cmd)) {
      const value = overrides.get(cmd);
      overrides.delete(cmd);
      return ok(value, arguments_);
    }

    const payload = (arguments_?.request as { payload?: Record<string, unknown> } | undefined)
      ?.payload;

    switch (cmd) {
      case 'chrona.temporal.commit_changes': {
        return ok({ id: 'c1' }, arguments_);
      }
      case 'chrona.temporal.list_commits': {
        return ok({ commits: [] }, arguments_);
      }
      case 'chrona.temporal.list_branches': {
        return ok({ branches: [{ name: 'main', head: 'c1' }] }, arguments_);
      }
      case 'chrona.temporal.diff': {
        return ok(
          {
            from: commitReferenceId(payload?.from) ?? 'from',
            to: commitReferenceId(payload?.to) ?? 'to',
            nodeAdds: 1,
            nodeMods: 0,
            nodeDels: 0,
            edgeAdds: 2,
            edgeMods: 0,
            edgeDels: 0,
          },
          arguments_,
        );
      }
      case 'chrona.temporal.topology_delta': {
        return ok(
          {
            from: commitReferenceId(payload?.from) ?? 'from',
            to: commitReferenceId(payload?.to) ?? 'to',
            nodeAdds: 2,
            nodeDels: 1,
            edgeAdds: 3,
            edgeDels: 1,
          },
          arguments_,
        );
      }
      case 'chrona.temporal.create_branch': {
        return ok(
          {
            name: (payload as { name?: string } | undefined)?.name ?? 'feature/x',
            head: 'c1',
          },
          arguments_,
        );
      }
      case 'chrona.temporal.merge_branches': {
        return ok({ result: 'merge-1' }, arguments_);
      }
      case 'praxis.metamodel.get': {
        return ok({ version: 'v1' }, arguments_);
      }
      default: {
        return ok(
          {
            asOf: commitReferenceId(payload?.asOf) ?? 'x',
            scenario: (payload as { scenario?: string } | undefined)?.scenario ?? undefined,
            confidence: (payload as { confidence?: number } | undefined)?.confidence ?? undefined,
            nodes: 0,
            edges: 0,
          },
          arguments_,
        );
      }
    }
  },
}));

describe('IpcTemporalAdapter', () => {
  beforeEach(() => {
    callLog.length = 0;
    overrides.clear();
  });

  it('stateAt/commit/list/create stubs roundtrip', async () => {
    const { IpcTemporalAdapter } = await import('adapters/timegraph-ipc');
    const a = new IpcTemporalAdapter();
    const asOf = 'c1';
    const s = await a.stateAt({ asOf });
    expect(s.asOf).toBe(asOf);
    expect(s.nodes).toBe(0);
    expect(s.edges).toBe(0);
    const diff = await a.diff({
      from: 'c0',
      to: asOf,
    });
    expect(diff.metrics.nodeAdds).toBe(1);
    expect(diff.metrics.edgeAdds).toBe(2);
    const topology = await a.topologyDelta({ from: 'c0', to: asOf });
    expect(topology.metrics.nodeAdds).toBe(2);
    expect(topology.metrics.edgeDels).toBe(1);
    const c = await a.commit({
      branch: 'main',
      message: 'seed',
      changes: { nodeCreates: ['n1'] },
    });
    expect(c.id).toBe('c1');
    const ls = await a.listCommits({ branch: 'main' });
    expect(Array.isArray(ls)).toBe(true);
    const branch = await a.createBranch({ name: 'feature/x', from: 'c1' });
    expect(branch.head).toBe('c1');
    const branches = await a.listBranches();
    expect(branches[0]?.name).toBe('main');
    const merge = await a.mergeBranches({ source: 'feature/x', target: 'main' });
    expect(merge.result).toBe('merge-1');
  });

  it('passes scope through diff IPC payload', async () => {
    const { IpcTemporalAdapter } = await import('adapters/timegraph-ipc');
    const a = new IpcTemporalAdapter();
    await a.diff({ from: 'c0', to: 'c1', scope: 'capability' });
    const diffCall = requireCall('chrona.temporal.diff');
    expect(callPayload(diffCall)).toEqual({
      from: { id: 'c0' },
      to: { id: 'c1' },
      scope: 'capability',
    });
  });

  it('omits optional fields when undefined and filters malformed payloads', async () => {
    const { IpcTemporalAdapter } = await import('adapters/timegraph-ipc');
    const a = new IpcTemporalAdapter();

    await a.stateAt({ asOf: 'c9', scenario: 'dev', confidence: 0.9 });
    const stateAtCall = requireCall('chrona.temporal.state_at');
    expect(callPayload(stateAtCall)).toEqual({
      asOf: { id: 'c9' },
      scenario: 'dev',
      confidence: 0.9,
    });

    await a.createBranch({ name: 'feature/no-from' });
    const createBranchCall = requireCall('chrona.temporal.create_branch');
    expect(callPayload(createBranchCall)).toEqual({ name: 'feature/no-from' });

    await a.stateAt({ asOf: 'c10' });
    const stateAtCallMinimal = callLog.filter(
      (entry) => entry.cmd === 'chrona.temporal.state_at',
    )[1];
    expect(callPayload(stateAtCallMinimal)).toEqual({ asOf: { id: 'c10' } });

    await a.diff({ from: 'c0', to: 'c1' });
    const diffCall = requireCall('chrona.temporal.diff');
    expect(callPayload(diffCall)).toEqual({ from: { id: 'c0' }, to: { id: 'c1' } });

    overrides.set('chrona.temporal.list_branches', {
      branches: [{ name: 123 }, { head: 'h1' }],
    });
    const branches = await a.listBranches();
    expect(branches).toEqual([
      { name: '', head: undefined },
      { name: '', head: 'h1' },
    ]);

    overrides.set('chrona.temporal.topology_delta', {
      from: 1,
      to: undefined,
      node_adds: '2',
      node_dels: 1,
      edge_dels: 0,
    });
    const delta = await a.topologyDelta({ from: 'c0', to: 'c1' });
    expect(delta).toEqual({
      from: '',
      to: '',
      metrics: { nodeAdds: 0, nodeDels: 1, edgeAdds: 0, edgeDels: 0 },
    });

    overrides.set('chrona.temporal.topology_delta', {
      from: 'c0',
      to: 'c1',
      node_adds: 2,
      node_dels: 0,
      edge_adds: 1,
      edge_dels: 4,
    });
    const deltaOk = await a.topologyDelta({ from: 'c0', to: 'c1' });
    expect(deltaOk.metrics).toEqual({ nodeAdds: 2, nodeDels: 0, edgeAdds: 1, edgeDels: 4 });
  });

  it('normalises merge conflicts and tolerates missing fields', async () => {
    const { IpcTemporalAdapter } = await import('adapters/timegraph-ipc');
    const a = new IpcTemporalAdapter();

    overrides.set('chrona.temporal.merge_branches', {
      result: 123,
      conflicts: [
        { reference: 'r1', kind: 'node', message: 'conflict' },
        { reference: '', kind: 'edge' },
        { kind: 'node' },
      ],
    });

    const result = await a.mergeBranches({ source: 'a', target: 'b' });
    expect(result.result).toBeUndefined();
    expect(result.conflicts).toEqual([{ reference: 'r1', kind: 'node', message: 'conflict' }]);
  });

  it('serializes commit payloads and supports meta-model fetch', async () => {
    const { IpcTemporalAdapter } = await import('adapters/timegraph-ipc');
    const a = new IpcTemporalAdapter();

    await a.commit({
      branch: 'dev',
      parent: 'p0',
      author: 'Alice',
      message: 'Full changes',
      tags: ['t1'],
      time: '2025-01-01T00:00:00Z',
      changes: {
        nodeCreates: [],
        nodeDeletes: ['n1'],
        edgeCreates: [{ from: 'n1', to: 'n2' }],
        edgeDeletes: [{ from: 'n2', to: 'n3' }],
      },
    });

    const commitCall = requireCall('chrona.temporal.commit_changes');
    const requestId = callRequestId(commitCall);
    expect(requestId).not.toBeUndefined();
    expect(callPayload(commitCall)).toEqual({
      branch: 'dev',
      parent: 'p0',
      author: 'Alice',
      message: 'Full changes',
      tags: ['t1'],
      time: '2025-01-01T00:00:00Z',
      changes: {
        nodeDeletes: [{ id: 'n1' }],
        edgeCreates: [{ from: 'n1', to: 'n2' }],
        edgeDeletes: [{ from: 'n2', to: 'n3' }],
      },
    });

    const meta = await a.getMetaModel();
    expect(meta).toEqual({ version: 'v1' });
  });
});
