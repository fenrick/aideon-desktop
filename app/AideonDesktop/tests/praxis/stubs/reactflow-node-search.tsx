import type { ReactNode } from 'react';

/**
 *
 * @param root0
 * @param root0.children
 */
export function NodeSearchDialog({ children }: Readonly<{ children?: ReactNode }>) {
  return <div data-testid="node-search-dialog">{children}</div>;
}
