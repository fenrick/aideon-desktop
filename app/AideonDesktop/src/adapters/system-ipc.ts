import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

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
  await invoke('set_complete', { task });
}

/**
 *
 */
export async function openStyleguideWindow(): Promise<void> {
  await invoke('open_styleguide');
}
