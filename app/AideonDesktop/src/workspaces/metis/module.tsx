import type { WorkspaceModule, WorkspaceNavigationProperties } from 'workspaces/types';

import { Card, CardContent, CardHeader, CardTitle } from 'design-system/components/ui/card';

/**
 *
 * @param root0
 * @param root0.message
 */
function ComingSoonCard({ message }: { readonly message?: string }) {
  return (
    <div className="p-6">
      <Card>
        <CardHeader>
          <CardTitle>Coming soon</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">This workspace is not ready yet.</p>
          {message ? <p className="text-xs text-muted-foreground">{message}</p> : undefined}
        </CardContent>
      </Card>
    </div>
  );
}

/**
 *
 * @param _
 */
function ComingSoonNavigation(_: Readonly<WorkspaceNavigationProperties>) {
  return <ComingSoonCard />;
}

/**
 *
 */
function ComingSoonContent() {
  return <ComingSoonCard />;
}

/**
 *
 */
function ComingSoonInspector() {
  return <ComingSoonCard />;
}

export const METIS_WORKSPACE: WorkspaceModule = {
  id: 'metis',
  label: 'Metis',
  enabled: false,
  Navigation: ComingSoonNavigation,
  Content: ComingSoonContent,
  Inspector: ComingSoonInspector,
};
