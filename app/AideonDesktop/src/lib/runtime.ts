/**
 * Detect whether the current execution environment has a window.
 */
export function isBrowserRuntime(): boolean {
  return 'window' in globalThis;
}

/**
 * Detect whether the code is running inside a Tauri runtime.
 */
export function isTauriRuntime(): boolean {
  if (!isBrowserRuntime()) {
    return false;
  }
  const runtime = globalThis as { __TAURI__?: unknown; __TAURI_INTERNALS__?: unknown };
  return Boolean(runtime.__TAURI__ ?? runtime.__TAURI_INTERNALS__);
}

/**
 * Detect whether the build is a development build.
 */
export function isDevelopmentBuild(): boolean {
  return process.env.NODE_ENV !== 'production';
}
