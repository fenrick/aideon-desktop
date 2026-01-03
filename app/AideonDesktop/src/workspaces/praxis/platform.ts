/**
 * Detect whether the renderer is executing inside a Tauri host.
 * @returns {boolean} `true` when Tauri internals are present on `window`; otherwise `false`.
 */
export function isTauri(): boolean {
  if (!('window' in globalThis)) {
    return false;
  }
  const win = globalThis.window as Window | undefined;
  return Boolean(win?.__TAURI_INTERNALS__ ?? win?.__TAURI_METADATA__);
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
    __TAURI_METADATA__?: unknown;
  }
}
