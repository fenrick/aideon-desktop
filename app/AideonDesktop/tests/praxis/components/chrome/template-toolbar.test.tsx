import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import type { CanvasTemplate } from 'praxis/templates';

/**
 * Dynamically import the subject after applying module mocks.
 */
async function loadTemplateToolbar() {
  const loadedModule = await import('praxis/components/chrome/template-toolbar');
  return loadedModule.TemplateToolbar;
}

describe('TemplateToolbar', () => {
  it('renders header and triggers save/add actions', async () => {
    vi.resetModules();
    vi.doMock('praxis/platform', () => ({
      isTauri: () => false,
    }));

    const TemplateToolbar = await loadTemplateToolbar();
    const onTemplateSave = vi.fn();
    const onCreateWidget = vi.fn();
    const onTemplateChange = vi.fn();
    const templates: CanvasTemplate[] = [
      { id: 't1', name: 'T1', description: 'Desc' } as unknown as CanvasTemplate,
    ];

    render(
      <TemplateToolbar
        scenarioName="Scenario A"
        templateName="My Template"
        templates={templates}
        activeTemplateId="t1"
        onTemplateChange={onTemplateChange}
        onTemplateSave={onTemplateSave}
        onCreateWidget={onCreateWidget}
      />,
    );

    expect(screen.getByText(/Scenario A · My Template/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(onTemplateSave).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole('button', { name: 'Add widget' }));
    expect(onCreateWidget).toHaveBeenCalledTimes(1);
  });

  it('uses a native select in Tauri mode', async () => {
    vi.resetModules();
    vi.doMock('praxis/platform', () => ({
      isTauri: () => true,
    }));

    const TemplateToolbar = await loadTemplateToolbar();
    const onTemplateSave = vi.fn();
    const onCreateWidget = vi.fn();
    const onTemplateChange = vi.fn();
    const templates: CanvasTemplate[] = [
      { id: 't1', name: 'Template One', description: 'Desc' } as unknown as CanvasTemplate,
      { id: 't2', name: 'Template Two', description: 'Desc' } as unknown as CanvasTemplate,
    ];

    render(
      <TemplateToolbar
        scenarioName="Scenario A"
        templateName="My Template"
        templates={templates}
        activeTemplateId="t1"
        onTemplateChange={onTemplateChange}
        onTemplateSave={onTemplateSave}
        onCreateWidget={onCreateWidget}
      />,
    );

    const select = screen.getByLabelText('Select template');
    expect(select.tagName.toLowerCase()).toBe('select');

    fireEvent.change(select, { target: { value: 't2' } });
    expect(onTemplateChange).toHaveBeenCalledWith('t2');
  });
});
