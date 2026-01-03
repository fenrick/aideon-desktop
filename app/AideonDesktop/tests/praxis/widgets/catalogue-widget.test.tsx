import { render, screen, waitFor } from '@testing-library/react';
import type * as PraxisApi from 'praxis/praxis-api';
import type { PraxisCatalogueWidgetConfig as CatalogueWidgetConfig } from 'praxis/types';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const getCatalogueViewMock = vi.fn<typeof PraxisApi.getCatalogueView>();

vi.mock('praxis/praxis-api', async () => {
  const actual = await vi.importActual<typeof PraxisApi>('praxis/praxis-api');
  return {
    ...actual,
    getCatalogueView: (...arguments_: Parameters<typeof actual.getCatalogueView>) =>
      getCatalogueViewMock(...arguments_),
  };
});

import { CatalogueWidget } from 'praxis/widgets/catalogue-widget';

const CATALOGUE_WIDGET: CatalogueWidgetConfig = {
  id: 'catalogue-widget',
  kind: 'catalogue',
  title: 'Capability Catalogue',
  view: {
    id: 'catalogue-view',
    name: 'Capabilities',
    kind: 'catalogue',
    asOf: '2025-10-01T00:00:00.000Z',
    columns: [
      { id: 'name', label: 'Name', type: 'string' },
      { id: 'owner', label: 'Owner', type: 'string' },
    ],
  },
};

const CATALOGUE_VIEW: PraxisApi.CatalogueViewModel = {
  metadata: {
    id: 'catalogue-view',
    name: 'Capabilities',
    asOf: '2025-10-01T00:00:00.000Z',
    fetchedAt: '2025-10-01T00:01:00.000Z',
    scenario: 'main',
    source: 'mock',
  },
  columns: CATALOGUE_WIDGET.view.columns,
  rows: [
    { id: 'cap-1', values: { name: 'Customer Onboarding', owner: 'Ops' } },
    { id: 'cap-2', values: { name: 'Customer Support', owner: 'CX' } },
  ],
};

describe('CatalogueWidget', () => {
  beforeEach(() => {
    getCatalogueViewMock.mockReset();
  });

  it('renders catalogue rows and highlights external selection', async () => {
    getCatalogueViewMock.mockResolvedValue(CATALOGUE_VIEW);

    render(
      <CatalogueWidget
        widget={CATALOGUE_WIDGET}
        reloadVersion={0}
        selection={{ nodeIds: ['cap-1'], edgeIds: [], sourceWidgetId: 'graph-widget' }}
      />,
    );

    await waitFor(() => {
      expect(getCatalogueViewMock).toHaveBeenCalled();
    });
    expect(getCatalogueViewMock).toHaveBeenCalledWith(
      expect.objectContaining({
        asOf: CATALOGUE_WIDGET.view.asOf,
      }),
    );
    expect(screen.getByText('Customer Onboarding')).toBeInTheDocument();
    const selectedRow = screen.getByText('Customer Onboarding').closest('tr');
    expect(selectedRow).not.toBeNull();
    expect(selectedRow?.dataset.state).toBe('selected');
  });

  it('shows API errors instead of an empty placeholder', async () => {
    getCatalogueViewMock.mockRejectedValue(new Error('IPC offline'));

    render(<CatalogueWidget widget={CATALOGUE_WIDGET} reloadVersion={2} />);

    expect(await screen.findByText('IPC offline')).toBeInTheDocument();
  });
});
