import { invoke } from '@tauri-apps/api/core';

export interface IpcRequest<Payload> {
  readonly requestId: string;
  readonly payload: Payload;
}

export interface IpcError {
  readonly code: string;
  readonly message: string;
  readonly details: unknown;
}

export interface IpcResponse<Result> {
  readonly requestId: string;
  readonly status: 'ok' | 'error';
  readonly result?: Result;
  readonly error?: IpcError;
}

export class HostIpcError extends Error {
  public readonly code: string;
  public readonly details: unknown;

  /**
   * @param code - Stable machine-readable code.
   * @param message - Human-readable message.
   * @param details - Arbitrary error detail payload.
   */
  public constructor(code: string, message: string, details: unknown) {
    super(message);
    this.name = 'HostIpcError';
    this.code = code;
    this.details = details;
  }
}

let requestIdCounter = 0;

/** Generate a request ID suitable for correlating IPC responses. */
function nextRequestId(): string {
  requestIdCounter += 1;
  return `${crypto.randomUUID()}-${requestIdCounter.toString(36)}`;
}

/**
 * Invoke a host IPC command using the request/response envelope contract.
 * @param command - Namespaced command string.
 * @param payload - Command payload object.
 */
export async function invokeIpc<Result>(
  command: string,
  payload: Record<string, unknown>,
): Promise<Result> {
  const requestId = nextRequestId();
  const response = await invoke(command, {
    request: { requestId, payload } satisfies IpcRequest<Record<string, unknown>>,
  });

  if (typeof response !== 'object' || response === null) {
    throw new HostIpcError('invalid_response', 'Host returned invalid response.', response);
  }

  const record = response as Record<string, unknown>;
  const responseRequestId = record.requestId;
  if (typeof responseRequestId !== 'string') {
    throw new HostIpcError('invalid_response', 'Host returned invalid response.', record);
  }
  if (responseRequestId !== requestId) {
    throw new HostIpcError('invalid_response', 'Host returned mismatched requestId.', {
      expected: requestId,
      got: responseRequestId,
    });
  }

  const status = record.status;
  if (status === 'ok') {
    return record.result as Result;
  }

  if (status !== 'error') {
    throw new HostIpcError('invalid_response', 'Host returned invalid response.', record);
  }

  const error = record.error as Partial<IpcError> | undefined;
  const code = typeof error?.code === 'string' ? error.code : 'unknown_error';
  const message =
    typeof error?.message === 'string' ? error.message : 'Host reported an unknown error.';
  const details = error?.details ?? {};
  throw new HostIpcError(code, message, details);
}
