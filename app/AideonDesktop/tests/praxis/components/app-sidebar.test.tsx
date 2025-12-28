import { render, screen } from '@testing-library/react';
import type { ScenarioSummary } from 'praxis/praxis-api';
import { describe, expect, it } from 'vitest';

import { SidebarProvider } from 'design-system/components/ui/sidebar';
import { AppSidebar } from 'praxis/components/app-sidebar';

const SCENARIOS: ScenarioSummary[] = [
  {
    id: 'chronaplay',
    name: 'Chronaplay',
    description: 'Scenario sandbox',
    branch: 'chronaplay',
    updatedAt: '',
    isDefault: false,
  },
  {
    id: 'main',
    name: 'Mainline',
    description: 'Primary plan',
    branch: 'main',
    updatedAt: '',
    isDefault: true,
  },
];

describe('AppSidebar', () => {
  it('highlights the default scenario and shows nav controls', () => {
    render(
      <SidebarProvider>
        <AppSidebar scenarios={SCENARIOS} loading={false} />
      </SidebarProvider>,
    );

    expect(screen.getByText('Overview')).toBeInTheDocument();
    expect(screen.getByText('Mainline')).toBeInTheDocument();
    expect(screen.getByText('Primary plan')).toBeInTheDocument();
    expect(screen.getByText(/main - updated recently/i)).toBeInTheDocument();
  });

  it('indicates when scenarios are loading and no data exists yet', () => {
    render(
      <SidebarProvider>
        <AppSidebar scenarios={[]} loading />
      </SidebarProvider>,
    );

    expect(screen.getByText('Resolving scenario...')).toBeInTheDocument();
    expect(screen.getByText('Loading scenario data...')).toBeInTheDocument();
    expect(screen.getByText('Add a scenario to begin')).toBeInTheDocument();
  });
});
