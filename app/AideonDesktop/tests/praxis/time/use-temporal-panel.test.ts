import { act, createElement, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type {
  StateAtSnapshot,
  TemporalBranchSummary,
  TemporalCommitSummary,
  TemporalDiffSnapshot,
  TemporalMergeResult,
} from 'praxis/praxis-api';
import type { TemporalPanelActions, TemporalPanelState } from 'praxis/time/use-temporal-panel';

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/**
 * Forces pending microtasks to flush within an act block.
 * @returns {Promise<void>} resolves after microtasks flush
 */
async function flushMicrotasks(): Promise<void> {
  await act(async () => {
    await Promise.resolve();
  });
}

/**
 * Waits until a predicate succeeds or retries are exhausted.
 * @param {() => boolean} predicate condition to satisfy
 * @param {number} retries maximum attempts (default 10)
 */
async function waitForState(predicate: () => boolean, retries = 10): Promise<void> {
  for (let attempt = 0; attempt < retries; attempt += 1) {
    await flushMicrotasks();
    if (predicate()) {
      return;
    }
  }
  throw new Error('State condition not met');
}

type BranchesMock = () => Promise<TemporalBranchSummary[]>;
type CommitsMock = (branch: string) => Promise<TemporalCommitSummary[]>;
type SnapshotMock = (request: { asOf: string; scenario?: string }) => Promise<StateAtSnapshot>;
type DiffMock = (request: { from: string; to: string }) => Promise<TemporalDiffSnapshot>;
type MergeMock = (request: {
  source: string;
  target: string;
  strategy?: string;
}) => Promise<TemporalMergeResult>;

const listBranchesSpy = vi.fn<BranchesMock>();
const listCommitsSpy = vi.fn<CommitsMock>();
const getSnapshotSpy = vi.fn<SnapshotMock>();
const getDiffSpy = vi.fn<DiffMock>();
const mergeSpy = vi.fn<MergeMock>();

vi.mock('praxis/praxis-api', () => ({
  listTemporalBranches: (...arguments_: Parameters<typeof listBranchesSpy>) =>
    listBranchesSpy(...arguments_),
  listTemporalCommits: (...arguments_: Parameters<typeof listCommitsSpy>) =>
    listCommitsSpy(...arguments_),
  getStateAtSnapshot: (...arguments_: Parameters<typeof getSnapshotSpy>) =>
    getSnapshotSpy(...arguments_),
  getTemporalDiff: (...arguments_: Parameters<typeof getDiffSpy>) => getDiffSpy(...arguments_),
  mergeTemporalBranches: (...arguments_: Parameters<typeof mergeSpy>) => mergeSpy(...arguments_),
}));

import { useTemporalPanel } from 'praxis/time/use-temporal-panel';

const MAIN_COMMITS: TemporalCommitSummary[] = [
  {
    id: 'commit-main-001',
    branch: 'main',
    parents: [],
    author: 'Chrona',
    message: 'Seed',
    tags: [],
    changeCount: 2,
  },
  {
    id: 'commit-main-002',
    branch: 'main',
    parents: ['commit-main-001'],
    author: 'Chrona',
    message: 'Initial rollout',
    tags: ['baseline'],
    changeCount: 5,
  },
];

const FEATURE_COMMITS: TemporalCommitSummary[] = [
  {
    id: 'commit-feature-001',
    branch: 'chronaplay',
    parents: ['commit-main-002'],
    author: 'Metis',
    message: 'Scenario punch list',
    tags: [],
    changeCount: 1,
  },
];

const SNAPSHOTS: Record<string, StateAtSnapshot> = {
  'commit-main-001': {
    asOf: 'commit-main-001',
    scenario: 'main',
    confidence: 0.9,
    nodes: 12,
    edges: 18,
  },
  'commit-main-002': {
    asOf: 'commit-main-002',
    scenario: 'main',
    confidence: 0.95,
    nodes: 20,
    edges: 28,
  },
  'commit-feature-001': {
    asOf: 'commit-feature-001',
    scenario: 'chronaplay',
    confidence: 0.8,
    nodes: 22,
    edges: 31,
  },
};
const SNAPSHOT_MAP = new Map<string, StateAtSnapshot>(Object.entries(SNAPSHOTS));

/**
 * Creates a harness exposing state and actions from useTemporalPanel.
 * @returns {{state: TemporalPanelState, actions: TemporalPanelActions, unmount: () => void}} harness helpers
 */
function renderTemporalPanelHook() {
  const container = document.createElement('div');
  document.body.append(container);
  const root = createRoot(container);
  let current: [TemporalPanelState, TemporalPanelActions] | undefined;

  /**
   * Bridge component that captures hook output and sends it to the harness.
   * @param {{onValue: (value: [TemporalPanelState, TemporalPanelActions]) => void}} props callback sink
   * @returns {ReturnType<typeof createElement>} inert element for rendering
   */
  function HookBridge({
    onValue,
  }: {
    onValue: (value: [TemporalPanelState, TemporalPanelActions]) => void;
  }) {
    const value = useTemporalPanel();
    useEffect(() => {
      onValue(value);
    }, [onValue, value]);
    return createElement('div');
  }

  act(() => {
    root.render(
      createElement(HookBridge, {
        onValue: (value) => {
          current = value;
        },
      }),
    );
  });

  const getCurrent = () => {
    if (!current) {
      throw new Error('Hook has not initialised yet');
    }
    return current;
  };

  return {
    get state() {
      return getCurrent()[0];
    },
    get actions() {
      return getCurrent()[1];
    },
    unmount() {
      act(() => {
        root.unmount();
      });
      container.remove();
    },
  };
}

describe('useTemporalPanel', () => {
  beforeEach(() => {
    listBranchesSpy.mockReset();
    listCommitsSpy.mockReset();
    getSnapshotSpy.mockReset();
    getDiffSpy.mockReset();
    mergeSpy.mockReset();

    listBranchesSpy.mockResolvedValue([
      { name: 'main', head: 'commit-main-002' },
      { name: 'chronaplay', head: 'commit-feature-001' },
    ]);

    listCommitsSpy.mockImplementation((branch: string) => {
      if (branch === 'main') {
        return Promise.resolve(MAIN_COMMITS);
      }
      if (branch === 'chronaplay') {
        return Promise.resolve(FEATURE_COMMITS);
      }
      return Promise.resolve([]);
    });

    getSnapshotSpy.mockImplementation(({ asOf }: { asOf: string; scenario?: string }) => {
      const snapshot = SNAPSHOT_MAP.get(asOf);
      if (!snapshot) {
        return Promise.reject(new Error(`Missing snapshot for ${asOf}`));
      }
      return Promise.resolve(snapshot);
    });

    getDiffSpy.mockResolvedValue({
      from: 'commit-main-001',
      to: 'commit-main-002',
      metrics: {
        nodeAdds: 1,
        nodeMods: 0,
        nodeDels: 0,
        edgeAdds: 1,
        edgeMods: 0,
        edgeDels: 0,
      },
    });

    mergeSpy.mockResolvedValue({ result: 'merged' });
  });

  it('loads branches, commits, and latest snapshot on mount', async () => {
    const harness = renderTemporalPanelHook();
    try {
      await waitForState(() => !harness.state.loading);

      expect(harness.state.branch).toBe('main');
      expect(harness.state.commitId).toBe('commit-main-002');
      expect(harness.state.snapshot).toEqual(SNAPSHOTS['commit-main-002']);
      expect(listBranchesSpy).toHaveBeenCalledTimes(1);
      expect(listCommitsSpy).toHaveBeenCalledWith('main');
    } finally {
      harness.unmount();
    }
  });

  it('fetches snapshots when selecting historical commits', async () => {
    const harness = renderTemporalPanelHook();
    try {
      await waitForState(() => harness.state.commitId === 'commit-main-002');

      getSnapshotSpy.mockClear();
      const snapshot = SNAPSHOTS['commit-main-001'];
      if (!snapshot) {
        throw new Error('Missing snapshot fixture.');
      }
      getSnapshotSpy.mockResolvedValueOnce(snapshot);

      act(() => {
        harness.actions.selectCommit('commit-main-001');
      });

      await waitForState(() => harness.state.commitId === 'commit-main-001');
      await waitForState(() => harness.state.snapshot?.asOf === 'commit-main-001');
      expect(getSnapshotSpy).toHaveBeenCalledWith({ asOf: 'commit-main-001', scenario: 'main' });
      const expectedSnapshot = SNAPSHOTS['commit-main-001'];
      if (!expectedSnapshot) {
        throw new Error('Missing snapshot fixture.');
      }
      expect(harness.state.snapshot).toEqual(expectedSnapshot);
    } finally {
      harness.unmount();
    }
  });

  it('surfaces merge conflicts when the worker rejects a merge', async () => {
    const harness = renderTemporalPanelHook();
    try {
      await waitForState(() => !harness.state.loading);

      act(() => {
        void harness.actions.selectBranch('chronaplay');
      });

      await waitForState(() => harness.state.branch === 'chronaplay');

      mergeSpy.mockResolvedValueOnce({
        result: 'conflicts',
        conflicts: [
          {
            reference: 'cap-customer-onboarding',
            kind: 'node',
            message: 'Conflicting capability',
          },
        ],
      });

      act(() => {
        void harness.actions.mergeIntoMain();
      });

      await waitForState(() => !harness.state.merging);
      expect(mergeSpy).toHaveBeenCalledWith({ source: 'chronaplay', target: 'main' });
      expect(harness.state.mergeConflicts).toEqual([
        {
          reference: 'cap-customer-onboarding',
          kind: 'node',
          message: 'Conflicting capability',
        },
      ]);
    } finally {
      harness.unmount();
    }
  });

  it('stores errors when branch loading fails', async () => {
    listBranchesSpy.mockRejectedValueOnce(new Error('network down'));

    const harness = renderTemporalPanelHook();
    try {
      await waitForState(() => !harness.state.loading);
      expect(harness.state.error).toContain('network down');
    } finally {
      harness.unmount();
    }
  });

  it('handles empty branches and avoids calling commits/snapshots', async () => {
    listBranchesSpy.mockResolvedValueOnce([]);

    const harness = renderTemporalPanelHook();
    try {
      await waitForState(() => !harness.state.loading);
      expect(harness.state.branch).toBeUndefined();
      expect(listCommitsSpy).not.toHaveBeenCalled();
      expect(getSnapshotSpy).not.toHaveBeenCalled();
    } finally {
      harness.unmount();
    }
  });

  it('surfaces errors when commit loading fails', async () => {
    listCommitsSpy.mockRejectedValueOnce(new Error('commit fetch failed'));

    const harness = renderTemporalPanelHook();
    try {
      await waitForState(() => !harness.state.loading);
      expect(harness.state.error).toContain('commit fetch failed');
    } finally {
      harness.unmount();
    }
  });

  it('does nothing when selecting commits without a branch or repeating the same commit', async () => {
    listBranchesSpy.mockResolvedValueOnce([]);

    const harness = renderTemporalPanelHook();
    try {
      await waitForState(() => !harness.state.loading);
      act(() => {
        harness.actions.selectCommit('commit-main-001');
      });
      expect(getSnapshotSpy).not.toHaveBeenCalled();
    } finally {
      harness.unmount();
    }

    const harness2 = renderTemporalPanelHook();
    try {
      await waitForState(() => harness2.state.commitId === 'commit-main-002');
      getSnapshotSpy.mockClear();
      act(() => {
        harness2.actions.selectCommit('commit-main-002');
      });
      await flushMicrotasks();
      expect(getSnapshotSpy).not.toHaveBeenCalled();
    } finally {
      harness2.unmount();
    }
  });

  it('clears snapshots when selecting an empty commit id', async () => {
    const harness = renderTemporalPanelHook();
    try {
      await waitForState(() => harness.state.commitId === 'commit-main-002');
      act(() => {
        harness.actions.selectCommit();
      });
      await waitForState(() => harness.state.commitId === undefined);
      expect(harness.state.snapshot).toBeUndefined();
    } finally {
      harness.unmount();
    }
  });

  it('does not merge when already on main; merges and reloads when successful', async () => {
    const harness = renderTemporalPanelHook();
    try {
      await waitForState(() => harness.state.branch === 'main');
      act(() => {
        void harness.actions.mergeIntoMain();
      });
      await flushMicrotasks();
      expect(mergeSpy).not.toHaveBeenCalled();
    } finally {
      harness.unmount();
    }

    const harness2 = renderTemporalPanelHook();
    try {
      await waitForState(() => !harness2.state.loading);
      act(() => {
        void harness2.actions.selectBranch('chronaplay');
      });
      await waitForState(() => harness2.state.branch === 'chronaplay');

      mergeSpy.mockResolvedValueOnce({ result: 'merged', conflicts: [] });

      act(() => {
        void harness2.actions.mergeIntoMain();
      });

      await waitForState(() => !harness2.state.merging);
      expect(mergeSpy).toHaveBeenCalledWith({ source: 'chronaplay', target: 'main' });
      expect(listBranchesSpy.mock.calls.length).toBeGreaterThan(1);
      expect(harness2.state.mergeConflicts).toBeUndefined();
    } finally {
      harness2.unmount();
    }
  });
});
