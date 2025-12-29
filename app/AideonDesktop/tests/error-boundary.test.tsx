import { render, screen } from '@testing-library/react';
import type { ReactElement } from 'react';
import { describe, expect, it, vi } from 'vitest';

import { ErrorBoundary } from './error-boundary';

/**
 *
 */
function ExplodingChild(): ReactElement {
  throw new Error('boom');
}

describe('ErrorBoundary', () => {
  it('renders a recovery UI when a descendant throws', () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => false);
    render(
      <ErrorBoundary>
        <ExplodingChild />
      </ErrorBoundary>,
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Reload' })).toBeInTheDocument();
    consoleSpy.mockRestore();
  });
});
