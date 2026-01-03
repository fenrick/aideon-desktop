import { getCurrentWindow } from '@tauri-apps/api/window';

import { invokeIpc } from './ipc';

export type SetupTask = 'frontend' | 'backend';

export const SYSTEM_IPC_COMMANDS = {
  setupComplete: 'system.setup.complete',
  windowOpen: 'system.window.open',
} as const;

/**
 *
 */
export function getCurrentWindowLabel(): string | undefined {
  try {
    return getCurrentWindow().label;
  } catch {
    return;
  }
}

/**
 *
 * @param task
 */
export async function setSetupComplete(task: SetupTask): Promise<void> {
  await invokeIpc(SYSTEM_IPC_COMMANDS.setupComplete, { task });
}

/**
 *
 */
export async function openStyleguideWindow(): Promise<void> {
  await invokeIpc(SYSTEM_IPC_COMMANDS.windowOpen, { window: 'styleguide' });
}
