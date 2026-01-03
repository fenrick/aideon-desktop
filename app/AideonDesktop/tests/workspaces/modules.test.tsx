import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { METIS_WORKSPACE } from '@/workspaces/metis/module';
import { MNEME_WORKSPACE } from '@/workspaces/mneme/module';

describe('workspace modules', () => {
  afterEach(() => {
    cleanup();
  });
  it('renders the metis coming soon content', () => {
    const Content = METIS_WORKSPACE.Content;

    render(<Content />);

    expect(screen.getByText('Coming soon')).toBeInTheDocument();
    expect(screen.getByText('This workspace is not ready yet.')).toBeInTheDocument();
  });

  it('renders the mneme coming soon navigation', () => {
    const Navigation = MNEME_WORKSPACE.Navigation;

    render(
      <Navigation activeWorkspaceId="praxis" onWorkspaceSelect={vi.fn()} workspaceOptions={[]} />,
    );

    expect(screen.getByText('Coming soon')).toBeInTheDocument();
    expect(screen.getByText('This workspace is not ready yet.')).toBeInTheDocument();
  });
});
