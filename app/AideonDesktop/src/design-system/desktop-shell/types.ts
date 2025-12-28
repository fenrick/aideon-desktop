import type { ReactNode } from 'react';

export interface DesktopShellSlots {
  readonly tree: ReactNode;
  readonly toolbar: ReactNode;
  readonly main: ReactNode;
  readonly properties: ReactNode;
}
