import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { KeyboardShortcutsDialog } from 'aideon/shell/keyboard-shortcuts-dialog';

describe('KeyboardShortcutsDialog', () => {
  it('renders the default shortcuts table when open', () => {
    render(<KeyboardShortcutsDialog open onOpenChange={vi.fn()} />);

    expect(screen.getByText('Keyboard shortcuts')).toBeInTheDocument();
    expect(screen.getByText('Open…')).toBeInTheDocument();
    expect(screen.getByText('Command palette')).toBeInTheDocument();
    expect(screen.getAllByText('CmdOrCtrl').length).toBeGreaterThan(0);
  });
});
