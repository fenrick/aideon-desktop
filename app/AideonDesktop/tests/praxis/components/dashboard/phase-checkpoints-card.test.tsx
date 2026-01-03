import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { PhaseCheckpointsCard } from 'praxis/components/dashboard/phase-checkpoints-card';

describe('PhaseCheckpointsCard', () => {
  it('renders all phases with status styling', () => {
    render(<PhaseCheckpointsCard />);

    expect(screen.getByText('Bootstrap shell')).toBeInTheDocument();
    expect(screen.getByText('Charts + templates')).toBeInTheDocument();
    expect(screen.getAllByText(/complete/i)).toHaveLength(4);
    expect(screen.getByText(/in-progress/i)).toBeInTheDocument();
  });
});
