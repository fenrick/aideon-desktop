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

/**
 * Narrow unknown values to plain object records.
 * @param value
 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

/**
 * Create a successful IPC envelope response for the adapter boundary.
 * @param result
 */
function mockIpcOk(result: unknown) {
  return (_command: string, invokeArguments: unknown) => {
    const request = isRecord(invokeArguments) ? invokeArguments.request : undefined;
    const requestId =
      isRecord(request) && typeof request.requestId === 'string' ? request.requestId : 'req';
    return Promise.resolve({ requestId, status: 'ok', result });
  };
}

describe('domain-data branches', () => {
  it('falls back to built-ins when not running in Tauri', async () => {
    isTauriMock.mockReturnValue(false);
    listScenariosMock.mockResolvedValue([
      { id: 's1', name: 'Main', branch: 'main', updatedAt: '', isDefault: true },
    ]);

    const projects = await listProjectsWithScenarios();
    expect(invokeMock).not.toHaveBeenCalled();
    expect(projects).not.toHaveLength(0);
    const firstProject = projects[0];
    if (!firstProject) {
      throw new Error('Expected at least one project.');
    }
    expect(firstProject.id).toBe('default-project');

    const templates = await listTemplatesFromHost();
    expect(templates.length).toBeGreaterThan(0);
  });

  it('normalises host results and falls back on errors or empty payloads', async () => {
    isTauriMock.mockReturnValue(true);
    invokeMock.mockImplementationOnce(
      mockIpcOk([{ id: 'p1', name: 'Proj', scenarios: [] }]),
    );
    const projects = await listProjectsWithScenarios();
    expect(projects[0]).toMatchObject({ id: 'p1', name: 'Proj' });

    invokeMock.mockImplementationOnce(mockIpcOk([]));
    const templatesEmpty = await listTemplatesFromHost();
    expect(templatesEmpty.length).toBeGreaterThan(0);

    invokeMock.mockRejectedValueOnce(new Error('boom'));
    const projectsFallback = await listProjectsWithScenarios();
    expect(projectsFallback).not.toHaveLength(0);
    const fallbackProject = projectsFallback[0];
    if (!fallbackProject) {
      throw new Error('Expected fallback project.');
    }
    expect(fallbackProject.id).toBe('default-project');
  });
});
