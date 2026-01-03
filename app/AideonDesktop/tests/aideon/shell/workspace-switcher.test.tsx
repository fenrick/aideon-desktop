import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { WorkspaceSwitcher } from 'aideon/shell/workspace-switcher';

describe('WorkspaceSwitcher', () => {
  it('renders the current workspace label and emits selections', async () => {
    const onSelect = vi.fn();

    render(
      <WorkspaceSwitcher
        currentId="praxis"
        options={[
          { id: 'praxis', label: 'Praxis' },
          { id: 'chrona', label: 'Chrona' },
        ]}
        onSelect={onSelect}
      />,
    );

    expect(screen.getByText('Praxis')).toBeInTheDocument();

    fireEvent.pointerDown(screen.getByRole('button', { name: 'Switch workspace' }));
    fireEvent.click(await screen.findByText('Chrona'));

    expect(onSelect).toHaveBeenCalledWith('chrona');
  });
});
