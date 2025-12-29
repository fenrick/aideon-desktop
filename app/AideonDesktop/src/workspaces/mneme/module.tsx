import type { WorkspaceModule } from 'workspaces/types';

import { Card, CardContent, CardHeader, CardTitle } from 'design-system/components/ui/card';

function ComingSoon() {
  return (
    <div className="p-6">
      <Card>
        <CardHeader>
          <CardTitle>Coming soon</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">This workspace is not ready yet.</p>
        </CardContent>
      </Card>
    </div>
  );
}

export const MNEME_WORKSPACE: WorkspaceModule = {
  id: 'mneme',
  label: 'Mneme',
  enabled: false,
  Navigation: ComingSoon,
  Content: ComingSoon,
  Inspector: ComingSoon,
};
