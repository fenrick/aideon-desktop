import { useCallback, useEffect, useState } from 'react';

import { toErrorMessage } from 'praxis/lib/errors';
import { getWorkerHealth, type WorkerHealth } from 'praxis/praxis-api';

export interface WorkerHealthState {
  snapshot?: WorkerHealth;
  loading: boolean;
  error?: string;
}

export interface WorkerHealthActions {
  refresh(): Promise<void>;
}

const INITIAL_STATE: WorkerHealthState = {
  loading: true,
};

/**
 *
 */
export function useWorkerHealth(): [WorkerHealthState, WorkerHealthActions] {
  const [state, setState] = useState<WorkerHealthState>(INITIAL_STATE);

  const refresh = useCallback(async () => {
    setState((previous) => ({ ...previous, loading: true, error: undefined }));
    try {
      const snapshot = await getWorkerHealth();
      setState({ loading: false, snapshot });
    } catch (unknownError) {
      setState({ loading: false, error: toErrorMessage(unknownError) });
    }
  }, []);

  useEffect(() => {
    queueMicrotask(() => {
      refresh().catch((_ignoredError: unknown) => {
        return;
      });
    });
  }, [refresh]);

  return [state, { refresh }];
}
