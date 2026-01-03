import { renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('praxis/praxis-api', () => ({
  getWorkerHealth: vi.fn(),
}));

import { useWorkerHealth } from 'praxis/health/use-worker-health';
import { getWorkerHealth } from 'praxis/praxis-api';

describe('useWorkerHealth', () => {
  it('returns snapshot on success', async () => {
    vi.mocked(getWorkerHealth).mockResolvedValue({ ok: true, timestamp_ms: 99 });

    const { result } = renderHook(() => useWorkerHealth());

    await waitFor(() => {
      expect(result.current[0].loading).toBe(false);
    });
    expect(result.current[0].snapshot).toEqual({ ok: true, timestamp_ms: 99 });
  });

  it('surfaces error messages on failure', async () => {
    vi.mocked(getWorkerHealth).mockRejectedValue(new Error('down'));

    const { result } = renderHook(() => useWorkerHealth());

    await waitFor(() => {
      expect(result.current[0].loading).toBe(false);
    });
    expect(result.current[0].error).toContain('down');
    expect(result.current[0].snapshot).toBeUndefined();
  });
});
