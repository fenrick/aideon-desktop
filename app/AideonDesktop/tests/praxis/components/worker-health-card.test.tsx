import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('praxis/praxis-api', () => ({
  getWorkerHealth: vi.fn(),
}));

import { WorkerHealthCard } from 'praxis/components/dashboard/worker-health-card';
import { getWorkerHealth } from 'praxis/praxis-api';

describe('WorkerHealthCard', () => {
  it('renders operational status on healthy response', async () => {
    vi.mocked(getWorkerHealth).mockResolvedValue({ ok: true, timestamp_ms: 1234 });

    render(<WorkerHealthCard />);

    await waitFor(() => {
      expect(screen.getByText('Operational')).toBeInTheDocument();
    });
    expect(screen.getByText('Worker health')).toBeInTheDocument();
  });

  it('shows error state when health call fails', async () => {
    vi.mocked(getWorkerHealth).mockRejectedValue(new Error('disconnected'));

    render(<WorkerHealthCard />);

    await waitFor(() => {
      expect(screen.getByText(/disconnected/i)).toBeInTheDocument();
    });
    const refreshButtons = screen.getAllByText('Refresh');
    expect(refreshButtons[0]).toBeEnabled();
  });
});
