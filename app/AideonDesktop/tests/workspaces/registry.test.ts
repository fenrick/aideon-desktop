import { describe, expect, it } from 'vitest';

import { PRAXIS_WORKSPACE } from '@/workspaces/praxis/module';
import { getWorkspace, getWorkspaceOptions, WORKSPACES } from '@/workspaces/registry';
import type { WorkspaceId } from '@/workspaces/types';

describe('workspace registry', () => {
  it('returns a workspace by id', () => {
    expect(getWorkspace('mneme').id).toBe('mneme');
    expect(getWorkspace('metis').id).toBe('metis');
  });

  it('falls back to praxis for unknown ids', () => {
    expect(getWorkspace('unknown' as WorkspaceId)).toBe(PRAXIS_WORKSPACE);
  });

  it('maps navigation options', () => {
    const options = getWorkspaceOptions();

    expect(options).toHaveLength(WORKSPACES.length);
    expect(options).toContainEqual({
      id: 'mneme',
      label: 'Mneme',
      disabled: true,
    });
    expect(options).toContainEqual({
      id: 'praxis',
      label: 'Praxis',
      disabled: false,
    });
  });
});
