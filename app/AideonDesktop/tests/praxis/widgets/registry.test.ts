import { describe, expect, it } from 'vitest';

import { listWidgetRegistry, widgetDefaults } from 'praxis/widgets/registry';

describe('widget registry', () => {
  it('exposes defaults for each widget type', () => {
    const registry = listWidgetRegistry();
    expect(registry).toHaveLength(4);
    expect(widgetDefaults('graph')?.defaultView.kind).toBe('graph');
    expect(widgetDefaults('chart')?.defaultView.kind).toBe('chart');
  });
});
