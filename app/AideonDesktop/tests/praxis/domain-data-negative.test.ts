import { describe, expect, it, vi } from 'vitest';

describe('domain-data negative paths', () => {
  it('falls back to built-in templates when host invoke fails', async () => {
    const invokeMock = vi.fn().mockRejectedValue(new Error('no host'));
    vi.doMock('praxis/platform', () => ({ isTauri: () => true }));
    vi.doMock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

    const { listTemplatesFromHost } = await import('praxis/domain-data');
    const templates = await listTemplatesFromHost();
    expect(templates.length).toBeGreaterThan(0);
    expect(invokeMock).toHaveBeenCalled();
  });
});
