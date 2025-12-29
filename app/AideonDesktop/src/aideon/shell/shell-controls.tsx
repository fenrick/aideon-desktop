import { createContext, useContext, type PropsWithChildren } from 'react';

export interface AideonShellControls {
  readonly inspectorCollapsed: boolean;
  readonly toggleInspector: () => void;
}

const AideonShellControlsContext = createContext<AideonShellControls | undefined>(undefined);

export interface AideonShellControlsProviderProperties extends PropsWithChildren {
  readonly value: AideonShellControls;
}

/**
 * Provide desktop shell controls (pane toggles, visibility) to toolbar/menu components.
 * @param root0
 * @param root0.value
 * @param root0.children
 */
export function AideonShellControlsProvider({
  value,
  children,
}: AideonShellControlsProviderProperties) {
  return (
    <AideonShellControlsContext.Provider value={value}>
      {children}
    </AideonShellControlsContext.Provider>
  );
}

/**
 * Access shell-level controls when rendered inside `AideonDesktopShell`.
 */
export function useAideonShellControls(): AideonShellControls | undefined {
  return useContext(AideonShellControlsContext);
}
