import { useCallback, useEffect, useState } from 'react';

import {
  getStateAtSnapshot,
  getTemporalDiff,
  listTemporalBranches,
  listTemporalCommits,
  mergeTemporalBranches,
  type StateAtSnapshot,
  type TemporalBranchSummary,
  type TemporalCommitSummary,
  type TemporalDiffSnapshot,
  type TemporalMergeConflict,
} from 'praxis/praxis-api';

import { toErrorMessage } from 'praxis/lib/errors';

export interface TemporalPanelState {
  readonly branches: TemporalBranchSummary[];
  readonly branch?: string;
  readonly commits: TemporalCommitSummary[];
  readonly commitId?: string;
  readonly snapshot?: StateAtSnapshot;
  readonly loading: boolean;
  readonly snapshotLoading: boolean;
  readonly error?: string;
  readonly mergeConflicts?: TemporalMergeConflict[];
  readonly merging: boolean;
  readonly diff?: TemporalDiffSnapshot;
}

export interface TemporalPanelActions {
  readonly selectBranch: (branch: string) => Promise<void>;
  readonly selectCommit: (commitId?: string) => void;
  readonly refreshBranches: () => Promise<void>;
  readonly mergeIntoMain: () => Promise<void>;
}

const INITIAL_STATE: TemporalPanelState = {
  branches: [],
  commits: [],
  loading: true,
  snapshotLoading: false,
  mergeConflicts: undefined,
  merging: false,
  diff: undefined,
};

/**
 * Hook backing the temporal panel: timelines, moments, snapshots, apply-to-primary, and diff preview.
 * @returns {[TemporalPanelState, TemporalPanelActions]} current state and actions
 */
export function useTemporalPanel(): [TemporalPanelState, TemporalPanelActions] {
  const [state, setState] = useState<TemporalPanelState>(INITIAL_STATE);
  const loadDiff = useCallback(async (commits: TemporalCommitSummary[]) => {
    if (commits.length < 2) {
      setState((previous) => ({ ...previous, diff: undefined }));
      return;
    }
    const [fromCommit, toCommit] = commits.slice(-2) as [
      TemporalCommitSummary,
      TemporalCommitSummary,
    ];
    try {
      const diff = await getTemporalDiff({ from: fromCommit.id, to: toCommit.id });
      setState((previous) => ({ ...previous, diff }));
    } catch {
      setState((previous) => ({ ...previous, diff: undefined }));
    }
  }, []);

  const loadBranch = useCallback(
    async (branch: string) => {
      setState((previous) => ({
        ...previous,
        branch,
        commits: [],
        commitId: undefined,
        snapshot: undefined,
        loading: true,
        error: undefined,
        snapshotLoading: false,
        mergeConflicts: undefined,
      }));
      try {
        const commits = await listTemporalCommits(branch);
        const latest = commits.at(-1);
        let snapshot: StateAtSnapshot | undefined;
        if (latest) {
          snapshot = await getStateAtSnapshot({ asOf: latest.id, scenario: branch });
        }
        setState((previous) => ({
          ...previous,
          branch,
          commits,
          commitId: latest?.id,
          snapshot,
          snapshotLoading: false,
          loading: false,
          merging: false,
        }));
        await loadDiff(commits);
      } catch (unknownError) {
        setState((previous) => ({
          ...previous,
          loading: false,
          error: toErrorMessage(unknownError),
          snapshotLoading: false,
          merging: false,
        }));
      }
    },
    [loadDiff],
  );

  const loadBranches = useCallback(async () => {
    setState((previous) => ({ ...previous, loading: true, error: undefined }));
    try {
      const branches = await listTemporalBranches();
      const branch = pickInitialBranch(branches);
      setState((previous) => ({
        ...previous,
        branches,
        branch,
        mergeConflicts: undefined,
      }));
      if (branch) {
        await loadBranch(branch);
      } else {
        setState((previous) => ({ ...previous, loading: false, merging: false }));
      }
    } catch (unknownError) {
      setState((previous) => ({
        ...previous,
        loading: false,
        error: toErrorMessage(unknownError),
        mergeConflicts: undefined,
        merging: false,
      }));
    }
  }, [loadBranch]);

  const selectCommit = useCallback(
    (commitId?: string) => {
      const branch = state.branch;
      if (!branch) {
        return;
      }
      if (commitId !== undefined && commitId === state.commitId) {
        return;
      }
      if (!commitId) {
        setState((previous) => ({
          ...previous,
          commitId: undefined,
          snapshot: undefined,
          snapshotLoading: false,
          mergeConflicts: undefined,
        }));
        return;
      }
      setState((previous) => ({
        ...previous,
        commitId,
        snapshotLoading: true,
        error: undefined,
        mergeConflicts: undefined,
      }));
      const loadSnapshot = async () => {
        try {
          const snapshot = await getStateAtSnapshot({ asOf: commitId, scenario: branch });
          setState((previous) => ({
            ...previous,
            commitId,
            snapshot,
            snapshotLoading: false,
            loading: false,
          }));
        } catch (unknownError) {
          setState((previous) => ({
            ...previous,
            snapshotLoading: false,
            error: toErrorMessage(unknownError),
            mergeConflicts: undefined,
          }));
        }
      };
      loadSnapshot().catch((_ignoredError: unknown) => {
        return;
      });
    },
    [state.branch, state.commitId],
  );

  const refreshBranches = useCallback(async () => {
    await loadBranches();
  }, [loadBranches]);

  const mergeIntoMain = useCallback(async () => {
    const sourceBranch = state.branch;
    if (!sourceBranch || sourceBranch === 'main') {
      return;
    }
    setState((previous) => ({
      ...previous,
      merging: true,
      error: undefined,
      mergeConflicts: undefined,
    }));
    try {
      const result = await mergeTemporalBranches({ source: sourceBranch, target: 'main' });
      if (result.conflicts && result.conflicts.length > 0) {
        setState((previous) => ({
          ...previous,
          merging: false,
          mergeConflicts: result.conflicts,
          error: 'Merge requires manual resolution.',
        }));
        return;
      }
      await loadBranches();
      setState((previous) => ({
        ...previous,
        merging: false,
        mergeConflicts: undefined,
      }));
    } catch (unknownError) {
      setState((previous) => ({
        ...previous,
        merging: false,
        mergeConflicts: undefined,
        error: toErrorMessage(unknownError),
      }));
    }
  }, [loadBranches, state.branch]);

  useEffect(() => {
    loadBranches().catch((_ignoredError: unknown) => {
      return;
    });
  }, [loadBranches]);

  const selectBranchAction = useCallback(
    async (branch: string) => {
      await loadBranch(branch);
    },
    [loadBranch],
  );

  return [
    state,
    {
      selectBranch: selectBranchAction,
      selectCommit,
      refreshBranches,
      mergeIntoMain,
    },
  ];
}

/**
 * Picks the preferred initial timeline (main if available, otherwise first entry).
 * @param {TemporalBranchSummary[]} branches available timeline summaries
 * @returns {string | undefined} chosen timeline name
 */
export function pickInitialBranch(branches: TemporalBranchSummary[]): string | undefined {
  if (branches.length === 0) {
    return undefined;
  }
  const main = branches.find((branch) => branch.name === 'main');
  return main?.name ?? branches[0]?.name;
}
