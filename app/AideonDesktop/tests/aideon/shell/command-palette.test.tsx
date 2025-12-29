import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { AideonCommandPalette } from 'aideon/shell/command-palette';

Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
  value: vi.fn(),
  writable: true,
});

afterEach(() => {
  cleanup();
});

describe('AideonCommandPalette', () => {
  it('groups commands, sorts by label, and closes after selection', () => {
    const onOpenChange = vi.fn();
    const onSelect = vi.fn();

    render(
      <AideonCommandPalette
        open
        onOpenChange={onOpenChange}
        commands={[
          { id: 'z', group: 'Help', label: 'Zebra', onSelect },
          { id: 'a', group: 'Help', label: 'Apple', onSelect },
          { id: 'x', label: 'Ungrouped', onSelect },
        ]}
      />,
    );

    expect(screen.getByText('Help')).toBeInTheDocument();
    expect(screen.getByText('Commands')).toBeInTheDocument();

    const apple = screen.getByText('Apple');
    fireEvent.click(apple);

    expect(onSelect).toHaveBeenCalled();
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it('does not select disabled commands', () => {
    const onOpenChange = vi.fn();
    const onSelect = vi.fn();

    render(
      <AideonCommandPalette
        open
        onOpenChange={onOpenChange}
        commands={[{ id: 'x', label: 'Disabled', disabled: true, onSelect }]}
      />,
    );

    fireEvent.click(screen.getByText('Disabled'));
    expect(onSelect).not.toHaveBeenCalled();
    expect(onOpenChange).not.toHaveBeenCalled();
  });
});
