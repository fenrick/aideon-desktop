import { describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
const invoke = vi.mocked(await import('@tauri-apps/api/core').then((m) => m.invoke));
vi.mock('praxis/platform', () => ({ isTauri: () => true }));

describe('praxis-api state/diff host calls', () => {
  it('serializes optional fields for stateAt', async () => {
    invoke.mockResolvedValueOnce({
      asOf: '2025-01-01',
      scenario: undefined,
      confidence: undefined,
      nodes: 1,
      edges: 2,
    });
    const { getStateAtSnapshot } = await import('praxis/praxis-api');

    const snapshot = await getStateAtSnapshot({ asOf: '2025-01-01' });

    expect(invoke).toHaveBeenCalledWith('temporal_state_at', { payload: { asOf: '2025-01-01' } });
    expect(snapshot).toEqual({
      asOf: '2025-01-01',
      scenario: undefined,
      confidence: undefined,
      nodes: 1,
      edges: 2,
    });
  });

  it('passes scope into temporal diff payload', async () => {
    invoke.mockResolvedValueOnce({
      from: 'a',
      to: 'b',
      node_adds: 0,
      node_mods: 0,
      node_dels: 0,
      edge_adds: 0,
      edge_mods: 0,
      edge_dels: 0,
    });
    const { getTemporalDiff } = await import('praxis/praxis-api');

    await getTemporalDiff({ from: 'a', to: 'b', scope: 'Capability' });

    expect(invoke).toHaveBeenCalledWith('temporal_diff', {
      payload: { from: 'a', to: 'b', scope: 'Capability' },
    });
  });
});
