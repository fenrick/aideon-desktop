import { createContext, useCallback, useContext, useMemo, useReducer } from 'react';

import { dedupeIds } from 'aideon/canvas/selection';
import { EMPTY_SELECTION, type SelectionState, type WidgetSelection } from 'aideon/canvas/types';

export interface SelectionProperties {
  readonly name?: string;
  readonly dataSource?: string;
  readonly layout?: string;
  readonly description?: string;
}

interface SelectionStoreState {
  readonly selection: SelectionState;
  readonly properties: Record<string, SelectionProperties>;
  readonly version: number;
}

type Action =
  | { type: 'set'; selection: SelectionState }
  | { type: 'clear' }
  | { type: 'updateProps'; id: string; patch: SelectionProperties }
  | { type: 'resetProps'; id: string };

const INITIAL_STATE: SelectionStoreState = {
  selection: EMPTY_SELECTION,
  properties: {},
  version: 0,
};

/**
 *
 * @param state
 * @param action
 */
function reducer(state: SelectionStoreState, action: Action): SelectionStoreState {
  switch (action.type) {
    case 'set': {
      return { ...state, selection: action.selection, version: state.version + 1 };
    }
    case 'clear': {
      return { ...state, selection: EMPTY_SELECTION, version: state.version + 1 };
    }
    case 'updateProps': {
      const current = state.properties[action.id] ?? {};
      return {
        ...state,
        properties: { ...state.properties, [action.id]: { ...current, ...action.patch } },
        version: state.version + 1,
      };
    }
    case 'resetProps': {
      const rest = Object.fromEntries(
        Object.entries(state.properties).filter(([key]) => key !== action.id),
      );
      return { ...state, properties: rest, version: state.version + 1 };
    }
    default: {
      return state;
    }
  }
}

interface SelectionContextValue {
  readonly state: SelectionStoreState;
  readonly setSelection: (selection: SelectionState) => void;
  readonly setFromWidget: (selection: WidgetSelection) => void;
  readonly clear: () => void;
  readonly updateProperties: (id: string, patch: SelectionProperties) => void;
  readonly resetProperties: (id: string) => void;
}

const SelectionContext = createContext<SelectionContextValue | undefined>(undefined);

/**
 *
 * @param root0
 * @param root0.children
 */
export function SelectionProvider({ children }: { readonly children: React.ReactNode }) {
  const [state, dispatch] = useReducer(reducer, INITIAL_STATE);

  const setSelection = useCallback((selection: SelectionState) => {
    dispatch({ type: 'set', selection });
  }, []);

  const setFromWidget = useCallback((selection: WidgetSelection) => {
    dispatch({
      type: 'set',
      selection: {
        sourceWidgetId: selection.widgetId,
        nodeIds: dedupeIds(selection.nodeIds),
        edgeIds: dedupeIds(selection.edgeIds),
      },
    });
  }, []);

  const clear = useCallback(() => {
    dispatch({ type: 'clear' });
  }, []);

  const updateProperties = useCallback((id: string, patch: SelectionProperties) => {
    dispatch({ type: 'updateProps', id, patch });
  }, []);

  const resetProperties = useCallback((id: string) => {
    dispatch({ type: 'resetProps', id });
  }, []);

  const value = useMemo(
    () => ({ state, setSelection, setFromWidget, clear, updateProperties, resetProperties }),
    [clear, resetProperties, setFromWidget, setSelection, state, updateProperties],
  );

  return <SelectionContext.Provider value={value}>{children}</SelectionContext.Provider>;
}

/**
 *
 */
export function useSelectionStore(): SelectionContextValue {
  const context = useContext(SelectionContext);
  if (!context) {
    throw new Error('useSelectionStore must be used within a SelectionProvider');
  }
  return context;
}

/**
 *
 * @param selection
 */
export function deriveSelectionKind(
  selection: SelectionState,
): 'widget' | 'node' | 'edge' | 'none' {
  if (selection.nodeIds.length > 0) {
    return 'node';
  }
  if (selection.edgeIds.length > 0) {
    return 'edge';
  }
  if (selection.sourceWidgetId) {
    return 'widget';
  }
  return 'none';
}

/**
 *
 * @param selection
 */
export function primarySelectionId(selection: SelectionState): string | undefined {
  if (selection.nodeIds[0]) {
    return selection.nodeIds[0];
  }
  if (selection.edgeIds[0]) {
    return selection.edgeIds[0];
  }
  return selection.sourceWidgetId;
}
