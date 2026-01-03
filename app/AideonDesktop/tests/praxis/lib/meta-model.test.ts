import { describe, expect, it, vi } from 'vitest';

import { fetchMetaModel } from 'praxis/lib/meta-model';
import { getMetaModelDocument } from 'praxis/praxis-api';
import { isTauri } from 'praxis/platform';

vi.mock('praxis/platform', () => ({ isTauri: vi.fn(() => false) }));
vi.mock('praxis/praxis-api', () => ({ getMetaModelDocument: vi.fn() }));

describe('meta-model fetch', () => {
  it('returns a deterministic sample schema outside Tauri', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    const schema = await fetchMetaModel();
    expect(schema.types.length).toBeGreaterThan(0);
    expect(schema.relationships).not.toHaveLength(0);
    const relationship = schema.relationships[0];
    if (!relationship) {
      throw new Error('Expected at least one relationship.');
    }
    expect(relationship.from).toContain('Application');
  });

  it('delegates to host contract inside Tauri', async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(getMetaModelDocument).mockResolvedValue({
      version: 'v1',
      description: undefined,
      types: [],
      relationships: [],
      validation: undefined,
    });

    await expect(fetchMetaModel()).resolves.toMatchObject({ version: 'v1' });
    expect(getMetaModelDocument).toHaveBeenCalled();
  });
});
