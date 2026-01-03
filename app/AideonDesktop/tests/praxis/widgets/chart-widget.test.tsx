import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { getChartView } from 'praxis/praxis-api';
import { ChartWidget } from 'praxis/widgets/chart-widget';

vi.mock('praxis/praxis-api', () => ({
  getChartView: vi.fn(),
}));
const mockedGetChartView = vi.mocked(getChartView);

const baseDefinition = {
  id: 'view-1',
  name: 'Chart',
  kind: 'chart' as const,
  asOf: 'c1',
  chartType: 'kpi' as const,
  measure: 'm1',
};

const baseMetadata = {
  id: 'view-1',
  name: 'Chart',
  asOf: 'c1',
  fetchedAt: '2025-01-01T00:00:00Z',
  source: 'host' as const,
};

const widget = {
  id: 'w1',
  title: 'Chart',
  size: 'full' as const,
  kind: 'chart' as const,
  view: baseDefinition,
};

describe('ChartWidget', () => {
  afterEach(() => {
    vi.clearAllMocks();
    cleanup();
  });

  it('renders KPI summary', async () => {
    mockedGetChartView.mockResolvedValue({
      chartType: 'kpi',
      kpi: { value: 4200, trend: 'up', delta: 12, units: '%' },
      series: [],
      metadata: baseMetadata,
    });

    render(<ChartWidget widget={widget} reloadVersion={0} />);
    await waitFor(() => expect(screen.getByText(/4,200 %/)).toBeInTheDocument());
    expect(mockedGetChartView).toHaveBeenCalledWith(
      expect.objectContaining({
        asOf: baseDefinition.asOf,
      }),
    );
    expect(screen.getByText(/vs last span/i)).toBeInTheDocument();
  });

  it('renders KPI without delta and with a down trend', async () => {
    mockedGetChartView.mockResolvedValue({
      chartType: 'kpi',
      kpi: { value: 10, trend: 'down', units: undefined, delta: undefined },
      series: [],
      metadata: baseMetadata,
    });

    render(<ChartWidget widget={widget} reloadVersion={10} />);
    await waitFor(() => expect(screen.getByText(/10/)).toBeInTheDocument());
    expect(screen.queryByText(/vs last span/i)).not.toBeInTheDocument();
  });

  it('handles KPI models without a KPI payload', async () => {
    mockedGetChartView.mockResolvedValue({
      chartType: 'kpi',
      kpi: undefined,
      series: [],
      metadata: baseMetadata,
    });

    render(<ChartWidget widget={widget} reloadVersion={11} />);
    await waitFor(() => expect(screen.getByText(/No data/i)).toBeInTheDocument());
  });

  it('shows line chart series', async () => {
    mockedGetChartView.mockResolvedValue({
      chartType: 'line',
      series: [
        {
          id: 's1',
          label: 'Throughput',
          points: [
            { label: 'Jan', value: 10 },
            { label: 'Feb', value: 20 },
          ],
        },
      ],
      metadata: baseMetadata,
    });

    render(
      <ChartWidget
        widget={{ ...widget, view: { ...baseDefinition, chartType: 'line' } }}
        reloadVersion={1}
      />,
    );
    await waitFor(() => expect(screen.getByLabelText(/Throughput trend/)).toBeInTheDocument());
    expect(screen.getByText(/Jan:10/)).toBeInTheDocument();
  });

  it('shows no data when the line series is missing or empty', async () => {
    mockedGetChartView.mockResolvedValueOnce({
      chartType: 'line',
      series: [],
      metadata: baseMetadata,
    });

    render(
      <ChartWidget
        widget={{ ...widget, view: { ...baseDefinition, chartType: 'line' } }}
        reloadVersion={12}
      />,
    );
    await waitFor(() => expect(screen.getByText(/No data/i)).toBeInTheDocument());

    cleanup();

    mockedGetChartView.mockResolvedValueOnce({
      chartType: 'line',
      series: [{ id: 's1', label: 'Empty', points: [], color: '#000' }],
      metadata: baseMetadata,
    });

    render(
      <ChartWidget
        widget={{ ...widget, view: { ...baseDefinition, chartType: 'line' } }}
        reloadVersion={13}
      />,
    );
    await waitFor(() => expect(screen.getByLabelText(/Empty trend/)).toBeInTheDocument());
  });

  it('falls back to bar chart for other types', async () => {
    mockedGetChartView.mockResolvedValue({
      chartType: 'bar',
      series: [
        {
          id: 's1',
          label: 'Capex',
          color: '#123',
          points: [
            { label: 'Q1', value: 5 },
            { label: 'Q2', value: 15 },
          ],
        },
      ],
      metadata: baseMetadata,
    });

    render(
      <ChartWidget
        widget={{ ...widget, view: { ...baseDefinition, chartType: 'bar' } }}
        reloadVersion={2}
      />,
    );
    await waitFor(() => expect(screen.getByText('Q1')).toBeInTheDocument());
    expect(screen.getByText('Capex')).toBeInTheDocument();
  });

  it('shows no data for empty bar series and handles missing points', async () => {
    mockedGetChartView.mockResolvedValueOnce({
      chartType: 'bar',
      series: [],
      metadata: baseMetadata,
    });
    render(
      <ChartWidget
        widget={{ ...widget, view: { ...baseDefinition, chartType: 'bar' } }}
        reloadVersion={14}
      />,
    );
    await waitFor(() => expect(screen.getByText(/No data/i)).toBeInTheDocument());

    cleanup();

    mockedGetChartView.mockResolvedValueOnce({
      chartType: 'bar',
      series: [
        {
          id: 's1',
          label: 'Primary',
          points: [
            { label: 'Q1', value: 1000 },
            { label: 'Q2', value: 1 },
          ],
        },
        {
          id: 's2',
          label: 'Secondary',
          points: [{ label: 'Q1', value: 50 }],
        },
      ],
      metadata: baseMetadata,
    });
    render(
      <ChartWidget
        widget={{ ...widget, view: { ...baseDefinition, chartType: 'bar' } }}
        reloadVersion={15}
      />,
    );
    await waitFor(() => expect(screen.getByText('Primary')).toBeInTheDocument());
    expect(screen.getByText('Secondary')).toBeInTheDocument();
    expect(screen.getByText('Q2')).toBeInTheDocument();
  });

  it('renders errors from host', async () => {
    mockedGetChartView.mockRejectedValue(new Error('boom'));
    render(
      <ChartWidget
        widget={{ ...widget, view: { ...baseDefinition, chartType: 'kpi' } }}
        reloadVersion={3}
      />,
    );
    await waitFor(() => expect(screen.getByText(/boom/)).toBeInTheDocument());
  });
});
