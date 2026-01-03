import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('praxis/platform', () => ({ isTauri: vi.fn() }));
vi.mock('praxis/praxis-api', () => ({ listScenarios: vi.fn() }));

import { listProjectsWithScenarios, listTemplatesFromHost } from 'praxis/domain-data';
import { isTauri } from 'praxis/platform';
import { listScenarios } from 'praxis/praxis-api';
import { BUILT_IN_TEMPLATES } from 'praxis/templates';

const isTauriMock = vi.mocked(isTauri);
const listScenariosMock = vi.mocked(listScenarios);

describe('domain-data adapters', () => {
  beforeEach(() => {
    isTauriMock.mockReturnValue(false);
    listScenariosMock.mockResolvedValue([
      {
        id: 's1',
        name: 'Scenario 1',
        branch: 'main',
        updatedAt: '2025-01-01T00:00:00Z',
        isDefault: true,
      },
    ]);
  });

  it('falls back to default project when host unavailable', async () => {
    const projects = await listProjectsWithScenarios();
    expect(projects).toHaveLength(1);
    expect(projects[0]?.scenarios[0]?.id).toBe('s1');
  });

  it('returns built-in templates in mock mode', async () => {
    const templates = await listTemplatesFromHost();
    expect(templates[0]?.id).toBe(BUILT_IN_TEMPLATES[0]?.id);
  });
});
