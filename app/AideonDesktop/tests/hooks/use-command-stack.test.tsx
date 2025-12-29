import { act, renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { useCommandStack } from 'praxis/hooks/use-command-stack';

describe('useCommandStack', () => {
  it('records commands and supports undo/redo with async fallbacks', async () => {
    const undo = vi.fn().mockResolvedValue(undefined);
    const redo = vi.fn().mockResolvedValue(undefined);

    const { result } = renderHook(() => useCommandStack());

    act(() => {
      result.current.record({ label: 'cmd', undo, redo });
    });
    expect(result.current.canUndo).toBe(true);
    expect(result.current.canRedo).toBe(false);

    await act(async () => {
      result.current.undo();
      await Promise.resolve();
    });
    expect(undo).toHaveBeenCalled();
    expect(result.current.canUndo).toBe(false);
    expect(result.current.canRedo).toBe(true);

    await act(async () => {
      result.current.redo();
      await Promise.resolve();
    });
    expect(redo).toHaveBeenCalled();
    expect(result.current.canUndo).toBe(true);
    expect(result.current.canRedo).toBe(false);
  });

  it('clears history and future stacks', () => {
    const { result } = renderHook(() => useCommandStack());
    act(() => {
      result.current.record({ label: 'cmd', undo: vi.fn(), redo: vi.fn() });
      result.current.record({ label: 'cmd2', undo: vi.fn(), redo: vi.fn() });
      result.current.clear();
    });
    expect(result.current.canUndo).toBe(false);
    expect(result.current.canRedo).toBe(false);
  });
});
