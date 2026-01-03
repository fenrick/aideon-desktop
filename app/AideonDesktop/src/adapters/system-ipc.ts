import { getCurrentWindow } from '@tauri-apps/api/window';

import { invokeIpc } from './ipc';

export type SetupTask = 'frontend' | 'backend';

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
  await invokeIpc('system.setup.complete', { task });
}

/**
 *
 */
export async function openStyleguideWindow(): Promise<void> {
  await invokeIpc('system.window.open', { window: 'styleguide' });
}
