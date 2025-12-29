import { describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('praxis/platform', () => ({ isTauri: vi.fn() }));
vi.mock('praxis/praxis-api', () => ({ listScenarios: vi.fn() }));

import { invoke } from '@tauri-apps/api/core';
import { listProjectsWithScenarios, listTemplatesFromHost } from 'praxis/domain-data';
import { isTauri } from 'praxis/platform';
import { listScenarios } from 'praxis/praxis-api';

const invokeMock = vi.mocked(invoke);
const isTauriMock = vi.mocked(isTauri);
const listScenariosMock = vi.mocked(listScenarios);

describe('domain-data branches', () => {
  it('falls back to built-ins when not running in Tauri', async () => {
    isTauriMock.mockReturnValue(false);
    listScenariosMock.mockResolvedValue([
      { id: 's1', name: 'Main', branch: 'main', updatedAt: '', isDefault: true },
    ]);

    const projects = await listProjectsWithScenarios();
    expect(invokeMock).not.toHaveBeenCalled();
    expect(projects).not.toHaveLength(0);
    expect(projects[0]!.id).toBe('default-project');

    const templates = await listTemplatesFromHost();
    expect(templates.length).toBeGreaterThan(0);
  });

  it('normalises host results and falls back on errors or empty payloads', async () => {
    isTauriMock.mockReturnValue(true);
    invokeMock.mockResolvedValueOnce([{ id: 'p1', name: 'Proj', scenarios: [] }]);
    const projects = await listProjectsWithScenarios();
    expect(projects[0]).toMatchObject({ id: 'p1', name: 'Proj' });

    invokeMock.mockResolvedValueOnce([]);
    const templatesEmpty = await listTemplatesFromHost();
    expect(templatesEmpty.length).toBeGreaterThan(0);

    invokeMock.mockRejectedValueOnce(new Error('boom'));
    const projectsFallback = await listProjectsWithScenarios();
    expect(projectsFallback).not.toHaveLength(0);
    expect(projectsFallback[0]!.id).toBe('default-project');
  });
});
