import { describe, expect, it } from 'vitest';

import {
  BUILT_IN_TEMPLATES,
  captureTemplateFromWidgets,
  instantiateTemplate,
} from 'praxis/templates';

describe('templates', () => {
  it('instantiates templates with default timestamps when asOf is omitted', () => {
    const template = BUILT_IN_TEMPLATES[0];
    expect(template).toBeDefined();
    if (!template) {
      return;
    }
    const widgets = instantiateTemplate(template, {});
    expect(widgets).toHaveLength(template.widgets.length);
    for (const widget of widgets) {
      expect(widget.view.asOf).toBeTruthy();
    }
  });

  it('instantiates templates with provided asOf context', () => {
    const template = BUILT_IN_TEMPLATES[0];
    expect(template).toBeDefined();
    if (!template) {
      return;
    }
    const widgets = instantiateTemplate(template, { asOf: 'c1', scenario: 'main' });
    for (const widget of widgets) {
      expect(widget.view.asOf).toBe('c1');
      expect(widget.view.scenario).toBe('main');
    }
  });

  it('captures runtime widgets into a reusable template', () => {
    const template = BUILT_IN_TEMPLATES[0];
    expect(template).toBeDefined();
    if (!template) {
      return;
    }
    const widgets = instantiateTemplate(template, { scenario: 'main' });
    const captured = captureTemplateFromWidgets('Snapshot', 'Saved layout', widgets);
    expect(captured.widgets.length).toBeGreaterThan(0);
    const firstWidget = captured.widgets[0];
    if (!firstWidget) {
      return;
    }
    const templateFirst = template.widgets[0];
    if (!templateFirst) {
      return;
    }
    expect(firstWidget.kind).toBe(templateFirst.kind);
    expect('asOf' in (firstWidget as { view: Record<string, unknown> }).view).toBe(false);
  });
});
