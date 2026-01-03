import { useCallback, useMemo, useState } from 'react';

interface Command {
  readonly label: string;
  readonly redo: () => void | Promise<void>;
  readonly undo: () => void | Promise<void>;
}

export interface CommandStack {
  readonly canUndo: boolean;
  readonly canRedo: boolean;
  readonly record: (command: Command) => void;
  readonly undo: () => void;
  readonly redo: () => void;
  readonly clear: () => void;
}

/**
 * Simple in-memory command stack for undo/redo of UI actions.
 */
export function useCommandStack(): CommandStack {
  const [history, setHistory] = useState<Command[]>([]);
  const [future, setFuture] = useState<Command[]>([]);

  const record = useCallback((command: Command) => {
    setHistory((previous) => [...previous, command]);
    setFuture([]);
  }, []);

  const undo = useCallback(() => {
    setHistory((previous) => {
      if (previous.length === 0) {
        return previous;
      }
      const next = [...previous];
      const command = next.pop();
      if (command) {
        const result = command.undo();
        if (result && typeof (result as Promise<unknown>).then === 'function') {
          (result as Promise<unknown>).catch(() => {
            /* ignore undo error */
          });
        }
        setFuture((futureStack) => [command, ...futureStack]);
      }
      return next;
    });
  }, []);

  const redo = useCallback(() => {
    setFuture((previous) => {
      if (previous.length === 0) {
        return previous;
      }
      const [command, ...rest] = previous;
      if (command) {
        const result = command.redo();
        if (result && typeof (result as Promise<unknown>).then === 'function') {
          (result as Promise<unknown>).catch(() => {
            /* ignore redo error */
          });
        }
        setHistory((historyStack) => [...historyStack, command]);
      }
      return rest;
    });
  }, []);

  const clear = useCallback(() => {
    setHistory([]);
    setFuture([]);
  }, []);

  return useMemo(
    () => ({
      canUndo: history.length > 0,
      canRedo: future.length > 0,
      record,
      undo,
      redo,
      clear,
    }),
    [future.length, history.length, record, redo, undo, clear],
  );
}
