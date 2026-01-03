import { describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('praxis/platform', () => ({ isTauri: vi.fn() }));
vi.mock('praxis/praxis-api', () => ({ listScenarios: vi.fn() }));

import { invoke } from '@tauri-apps/api/core';
import { listProjectsWithScenarios, listTemplatesFromHost } from 'praxis/domain-data';
import { isTauri } from 'praxis/platform';
import { listScenarios } from 'praxis/praxis-api';
import { BUILT_IN_TEMPLATES } from 'praxis/templates';

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

describe('domain-data normalization', () => {
  it('falls back when host payload is not an array', async () => {
    isTauriMock.mockReturnValue(true);
    invokeMock.mockImplementationOnce(mockIpcOk({ unexpected: true } as unknown));
    listScenariosMock.mockResolvedValueOnce([
      { id: 's1', name: 'Main', branch: 'main', updatedAt: '', isDefault: true },
    ]);

    const projects = await listProjectsWithScenarios();
    expect(projects[0]?.id).toBe('default-project');
    expect(listScenariosMock).toHaveBeenCalled();
  });

  it('normalises missing ids/names using crypto.randomUUID when available', async () => {
    const randomUUID = vi.fn(() => 'uuid-1');
    Object.defineProperty(globalThis, 'crypto', {
      configurable: true,
      value: { randomUUID },
    });

    isTauriMock.mockReturnValue(true);
    invokeMock.mockImplementationOnce(
      mockIpcOk([{ name: '  Project X  ', scenarios: 'bad' }] as unknown),
    );

    const projects = await listProjectsWithScenarios();
    expect(projects).toHaveLength(1);
    expect(projects[0]?.id).toBe('project-uuid-1');
    expect(projects[0]?.name).toBe('Project X');
    expect(projects[0]?.scenarios).toEqual([]);
  });

  it('normalises templates and falls back to built-ins when widgets are missing', async () => {
    const randomUUID = vi.fn(() => 'uuid-2');
    Object.defineProperty(globalThis, 'crypto', {
      configurable: true,
      value: { randomUUID },
    });

    isTauriMock.mockReturnValue(true);
    invokeMock.mockImplementationOnce(
      mockIpcOk([{ description: undefined, widgets: [] }] as unknown),
    );

    const templates = await listTemplatesFromHost();
    expect(templates[0]?.id).toBe('template-uuid-2');
    expect(templates[0]?.name).toBe(BUILT_IN_TEMPLATES[0]?.name);
    expect(templates[0]?.widgets).toEqual(BUILT_IN_TEMPLATES[0]?.widgets);
  });
});
