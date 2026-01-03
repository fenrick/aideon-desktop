/**
 * Extract a best-effort message from object-like error values.
 * Supports `{ message }` and `{ code, message }` shapes.
 * @param value The object to inspect.
 * @returns The extracted message, if available.
 */
function messageFromObjectValue(value: object): string | undefined {
  const record = value as Record<string, unknown>;
  const message = record.message;

  if (typeof message === 'string' && message.trim()) {
    return message;
  }

  if (typeof record.code === 'string' && typeof message === 'string') {
    return message;
  }

  return undefined;
}

/**
 * Extract a non-empty message from an `Error` instance.
 * @param value The value to inspect.
 * @returns The extracted message, if available.
 */
function messageFromErrorInstance(value: unknown): string | undefined {
  if (!(value instanceof Error)) {
    return undefined;
  }

  if (value.message.trim()) {
    return value.message;
  }

  return undefined;
}

/**
 * Convert an unknown value into a JSON string, falling back on `String(...)`
 * for values that cannot be stringified (e.g., circular structures).
 * @param value The value to stringify.
 * @returns A best-effort string representation.
 */
function safeStringify(value: unknown): string {
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

/**
 * Convert an unknown thrown value into a user-friendly error message.
 * @param error Value captured from a thrown error or rejection.
 * @returns Best-effort message derived from the value.
 */
export function toErrorMessage(error: unknown): string {
  if (typeof error === 'object' && error !== null) {
    const message = messageFromObjectValue(error);
    if (message !== undefined) return message;
  }
  const messageFromError = messageFromErrorInstance(error);
  if (messageFromError !== undefined) return messageFromError;
  if (typeof error === 'string') {
    return error;
  }
  return safeStringify(error);
}
