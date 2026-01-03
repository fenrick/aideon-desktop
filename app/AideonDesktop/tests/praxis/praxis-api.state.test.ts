import { describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
const invoke = vi.mocked(await import('@tauri-apps/api/core').then((m) => m.invoke));
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

describe('praxis-api state/diff host calls', () => {
  it('serializes optional fields for stateAt', async () => {
    invoke.mockImplementationOnce(
      mockIpcOk({
        asOf: '2025-01-01',
        scenario: undefined,
        confidence: undefined,
        nodes: 1,
        edges: 2,
      }),
    );
    const { getStateAtSnapshot } = await import('praxis/praxis-api');

    const snapshot = await getStateAtSnapshot({ asOf: '2025-01-01' });

    const calls = (invoke as unknown as { mock: { calls: unknown[][] } }).mock.calls;
    const invokeArguments = findInvokeArguments(calls, 'chrona.temporal.state_at');
    expect(payloadFromInvokeArguments(invokeArguments)).toEqual({
      asOf: { id: '2025-01-01' },
      scenario: undefined,
      confidence: undefined,
    });
    expect(snapshot).toEqual({
      asOf: '2025-01-01',
      scenario: undefined,
      confidence: undefined,
      nodes: 1,
      edges: 2,
    });
  });

  it('passes scope into temporal diff payload', async () => {
    invoke.mockImplementationOnce(
      mockIpcOk({
        from: 'a',
        to: 'b',
        node_adds: 0,
        node_mods: 0,
        node_dels: 0,
        edge_adds: 0,
        edge_mods: 0,
        edge_dels: 0,
      }),
    );
    const { getTemporalDiff } = await import('praxis/praxis-api');

    await getTemporalDiff({ from: 'a', to: 'b', scope: 'Capability' });

    const calls = (invoke as unknown as { mock: { calls: unknown[][] } }).mock.calls;
    const invokeArguments = findInvokeArguments(calls, 'chrona.temporal.diff');
    expect(payloadFromInvokeArguments(invokeArguments)).toEqual({
      from: { id: 'a' },
      to: { id: 'b' },
      scope: 'Capability',
    });
  });
});
