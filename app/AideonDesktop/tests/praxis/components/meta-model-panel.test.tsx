import { fireEvent, render, screen } from '@testing-library/react';
import type * as MetaModelModule from 'praxis/lib/meta-model';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const fetchMetaModelSpy = vi.fn<typeof MetaModelModule.fetchMetaModel>();

vi.mock('praxis/lib/meta-model', async () => {
  const actual = await vi.importActual<typeof MetaModelModule>('praxis/lib/meta-model');
  return {
    ...actual,
    fetchMetaModel: (...arguments_: Parameters<typeof actual.fetchMetaModel>) =>
      fetchMetaModelSpy(...arguments_),
  };
});

import { MetaModelPanel } from 'praxis/components/dashboard/meta-model-panel';

const SAMPLE_SCHEMA: MetaModelModule.MetaModelSchema = {
  version: '1.0',
  description: 'Test schema',
  types: [
    {
      id: 'Capability',
      label: 'Capability',
      attributes: [{ name: 'owner', type: 'string', required: true }],
    },
  ],
  relationships: [
    {
      id: 'supports',
      label: 'Supports',
      from: ['Capability'],
      to: ['Capability'],
    },
  ],
};

describe('MetaModelPanel', () => {
  beforeEach(() => {
    fetchMetaModelSpy.mockReset();
    fetchMetaModelSpy.mockResolvedValue(SAMPLE_SCHEMA);
  });

  it('renders schema summary and attributes', async () => {
    render(<MetaModelPanel />);

    expect(await screen.findByText('Capability')).toBeInTheDocument();
    expect(screen.getByText('Supports')).toBeInTheDocument();
  });

  it('shows error banner if fetch fails and allows retry', async () => {
    fetchMetaModelSpy
      .mockRejectedValueOnce(new Error('network down'))
      .mockResolvedValueOnce(SAMPLE_SCHEMA);
    render(<MetaModelPanel />);

    expect(await screen.findByText('Failed to load meta-model')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Reload schema'));
    expect(await screen.findByText('Capability')).toBeInTheDocument();
  });
});
