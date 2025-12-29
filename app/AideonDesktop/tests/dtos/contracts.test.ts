import { describe, expect, expectTypeOf, test } from 'vitest';

import type {
  TemporalDiffParameters,
  TemporalDiffSnapshot,
  TemporalStateParameters,
  TemporalStateSnapshot,
  WorkerHealth,
} from '../../src/dtos';

/**
 * Runtime guard to verify temporal state snapshot shapes.
 * @param {unknown} value candidate snapshot object
 */
function assertTemporalStateSnapshot(value: unknown): asserts value is TemporalStateSnapshot {
  if (typeof value !== 'object' || value === null) {
    throw new Error('Snapshot must be an object');
  }
  const snapshot = value as Record<string, unknown>;
  if (typeof snapshot.asOf !== 'string') throw new Error('Missing asOf');
  if (typeof snapshot.nodes !== 'number') throw new Error('Missing nodes');
  if (typeof snapshot.edges !== 'number') throw new Error('Missing edges');
}

/**
 * Runtime guard ensuring WorkerHealth payloads match the contract.
 * @param {unknown} value candidate health object
 */
function assertWorkerHealth(value: unknown): asserts value is WorkerHealth {
  if (typeof value !== 'object' || value === null) {
    throw new Error('Health payload must be an object');
  }
  const record = value as Record<string, unknown>;
  if (typeof record.ok !== 'boolean') throw new Error('Missing ok flag');
  if (typeof record.timestamp_ms !== 'number') throw new Error('Missing timestamp_ms');
}

describe('contract DTOs', () => {
  test('temporal state payload matches contract', () => {
    const request: TemporalStateParameters = { asOf: 'commit-123', scenario: 'main' };
    expectTypeOf(request).toEqualTypeOf<TemporalStateParameters>();

    const json = JSON.stringify({
      asOf: 'commit-123',
      scenario: 'main',
      confidence: 0.92,
      nodes: 42,
      edges: 77,
    });
    const parsed = JSON.parse(json) as unknown;
    assertTemporalStateSnapshot(parsed);
    expect(parsed.scenario).toBe('main');
  });

  test('temporal diff payload mirrors schema', () => {
    const parameters: TemporalDiffParameters = { from: 'c1', to: 'c2', scope: 'capability' };
    expectTypeOf(parameters).toEqualTypeOf<TemporalDiffParameters>();

    const summary: TemporalDiffSnapshot = {
      from: 'c1',
      to: 'c2',
      metrics: {
        nodeAdds: 1,
        nodeMods: 0,
        nodeDels: 0,
        edgeAdds: 2,
        edgeMods: 1,
        edgeDels: 0,
      },
    };
    expectTypeOf(summary.metrics.nodeAdds).toEqualTypeOf<number>();
  });

  test('worker health payloads deserialize with optional message', () => {
    const payload: unknown = JSON.parse('{"ok":false,"message":"degraded","timestamp_ms":1700}');
    assertWorkerHealth(payload);
    expect(payload.message).toBe('degraded');
  });
});
