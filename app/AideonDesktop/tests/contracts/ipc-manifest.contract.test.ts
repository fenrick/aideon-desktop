/// <reference types="vite/client" />

import fs from 'node:fs/promises';

import { TIMEGRAPH_IPC_COMMANDS } from '@/adapters/timegraph-ipc';
import { SYSTEM_IPC_COMMANDS } from '@/adapters/system-ipc';
import { MNEME_IPC_COMMANDS } from '@/workspaces/mneme/mneme-api';
import { PRAXIS_DOMAIN_IPC_COMMANDS } from '@/workspaces/praxis/domain-data';
import { PRAXIS_IPC_COMMANDS } from '@/workspaces/praxis/praxis-api';
import { describe, expect, it } from 'vitest';

interface IpcManifest {
  readonly schemaVersion: number;
  readonly commands: string[];
}

/**
 * Flatten a command registry object into a list of command strings.
 * @param record registry map
 */
function flattenCommands(record: Record<string, string>): string[] {
  return Object.values(record);
}

describe('IPC contract manifest', () => {
  it('ensures renderer command strings exist in host manifest', async () => {
    const manifestRaw = await fs.readFile('docs/contracts/ipc-manifest.json', 'utf8');
    const manifest = JSON.parse(manifestRaw) as IpcManifest;
    const hostCommands = new Set(manifest.commands);

    const usedCommands = [
      ...flattenCommands(SYSTEM_IPC_COMMANDS),
      ...flattenCommands(TIMEGRAPH_IPC_COMMANDS),
      ...flattenCommands(PRAXIS_DOMAIN_IPC_COMMANDS),
      ...flattenCommands(PRAXIS_IPC_COMMANDS),
      ...flattenCommands(MNEME_IPC_COMMANDS),
    ];

    const missing = [...usedCommands].filter((command) => !hostCommands.has(command));
    expect(missing).toEqual([]);
  });
});
