import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { TemplateHeader } from 'praxis/components/template-screen/template-header';
import type { CanvasTemplate } from 'praxis/templates';

vi.mock('design-system/components/ui/select', () => {
  const Select = ({
    onValueChange,
    value,
  }: {
    onValueChange?: (v: string) => void;
    value?: string;
  }) => (
    <div>
      <button data-testid="template-select" onClick={() => onValueChange?.('t2')}>
        {value ?? 'select'}
      </button>
    </div>
  );
  const SelectTrigger = ({ children }: { children?: React.ReactNode }) => <div>{children}</div>;
  const SelectValue = (properties: Record<string, unknown>) => <div {...properties} />;
  const SelectContent = ({ children }: { children?: React.ReactNode }) => <div>{children}</div>;
  const SelectItem = ({ children }: { children?: React.ReactNode }) => <div>{children}</div>;
  return { Select, SelectTrigger, SelectValue, SelectContent, SelectItem };
});

describe('TemplateHeader', () => {
  const templates: CanvasTemplate[] = [
    { id: 't1', documentId: 'canvasdoc-t1', name: 'Baseline Template', description: 'First', widgets: [] },
    { id: 't2', documentId: 'canvasdoc-t2', name: 'Resilience Template', description: 'Second', widgets: [] },
  ];

  it('shows scenario + template names and triggers callbacks', () => {
    const handleChange = vi.fn();
    const handleSave = vi.fn();
    const handleCreate = vi.fn();

    render(
      <TemplateHeader
        scenarioName="Launch Readiness"
        templateName="Baseline Template"
        templateDescription="Snapshot of the active scenario"
        templates={templates}
        activeTemplateId="t1"
        onTemplateChange={handleChange}
        onTemplateSave={handleSave}
        onCreateWidget={handleCreate}
      />,
    );

    expect(screen.getAllByText(/baseline template/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/launch readiness/i)).toBeInTheDocument();
    expect(screen.getByText(/snapshot of the active scenario/i)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId('template-select'));
    expect(handleChange).toHaveBeenCalledWith('t2');

    fireEvent.click(screen.getByRole('button', { name: /save template/i }));
    expect(handleSave).toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: /create widget/i }));
    expect(handleCreate).toHaveBeenCalled();
  });
});
