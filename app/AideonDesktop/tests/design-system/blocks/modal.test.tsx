import { render, screen } from '@testing-library/react';
import type * as React from 'react';
import { describe, expect, it, vi } from 'vitest';

/**
 *
 * @param testId
 */
function withTestId(testId: string) {
  return function WithTestId({
    className,
    children,
    ...properties
  }: {
    className?: string;
    children?: React.ReactNode;
  }) {
    return (
      <div data-testid={testId} data-class={className} {...properties}>
        {children}
      </div>
    );
  };
}

vi.mock('design-system/ui/dialog', () => {
  const Dialog = (({ children, ...properties }: { children: React.ReactNode }) => (
    <div data-testid="dialog" {...properties}>
      {children}
    </div>
  )) as React.FC<{ children: React.ReactNode }> & { displayName?: string };
  Dialog.displayName = 'Dialog';
  const DialogContent = withTestId('dialog-content') as React.FC<{ children?: React.ReactNode }> & {
    displayName?: string;
  };
  DialogContent.displayName = 'DialogContent';
  const DialogDescription = withTestId(
    'dialog-description',
  ) as React.FC<{ children?: React.ReactNode }> & { displayName?: string };
  DialogDescription.displayName = 'DialogDescription';
  const DialogFooter = withTestId('dialog-footer') as React.FC<{ children?: React.ReactNode }> & {
    displayName?: string;
  };
  DialogFooter.displayName = 'DialogFooter';
  const DialogHeader = withTestId('dialog-header') as React.FC<{ children?: React.ReactNode }> & {
    displayName?: string;
  };
  DialogHeader.displayName = 'DialogHeader';
  const DialogTitle = withTestId('dialog-title') as React.FC<{ children?: React.ReactNode }> & {
    displayName?: string;
  };
  DialogTitle.displayName = 'DialogTitle';
  return {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
  };
});

import {
  Modal,
  ModalContent,
  ModalDescription,
  ModalFooter,
  ModalHeader,
  ModalTitle,
} from 'design-system/blocks/modal';

describe('Modal block', () => {
  it('forwards props and merges class names for content', () => {
    render(
      <Modal>
        <ModalContent className="extra">body</ModalContent>
      </Modal>,
    );
    const content = screen.getByTestId('dialog-content');
    expect(content).toHaveAttribute('data-class', expect.stringContaining('extra'));
    expect(content).toHaveTextContent('body');
  });

  it('renders header, title, description, and footer helpers', () => {
    render(
      <Modal>
        <ModalHeader className="head">
          <ModalTitle className="title">Title</ModalTitle>
          <ModalDescription className="desc">Details</ModalDescription>
        </ModalHeader>
        <ModalFooter className="foot">Actions</ModalFooter>
      </Modal>,
    );
    expect(screen.getByTestId('dialog-header')).toHaveAttribute('data-class', expect.any(String));
    expect(screen.getByText('Title')).toBeInTheDocument();
    expect(screen.getByText('Details')).toBeInTheDocument();
    expect(screen.getByTestId('dialog-footer')).toHaveTextContent('Actions');
  });
});
