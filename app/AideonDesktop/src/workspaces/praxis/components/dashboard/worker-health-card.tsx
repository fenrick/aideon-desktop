import { Alert, AlertDescription, AlertTitle } from 'design-system/components/ui/alert';
import { Badge } from 'design-system/components/ui/badge';
import { Button } from 'design-system/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from 'design-system/components/ui/card';
import { HeartPulse, RefreshCw } from 'lucide-react';
import { useWorkerHealth } from 'praxis/health/use-worker-health';

/**
 *
 */
export function WorkerHealthCard() {
  const [state, actions] = useWorkerHealth();

  const timestamp = state.snapshot
    ? new Date(state.snapshot.timestamp_ms).toLocaleTimeString()
    : 'N/A';
  let badgeLabel = 'Needs attention';
  let badgeVariant: 'secondary' | 'default' | 'outline' = 'outline';
  if (state.loading) {
    badgeLabel = 'Checking…';
    badgeVariant = 'secondary';
  } else if (state.snapshot?.ok) {
    badgeLabel = 'Operational';
    badgeVariant = 'default';
  }
  const description =
    state.error ?? state.snapshot?.notes ?? 'Engine responds with mock data when Tauri is offline.';

  return (
    <Card>
      <CardHeader>
        <CardTitle>Worker health</CardTitle>
        <CardDescription>Rust engine status via Tauri IPC.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <Alert variant={state.error ? 'destructive' : 'default'} className="space-y-2">
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center gap-2">
              <HeartPulse className="h-4 w-4" />
              <AlertTitle className="text-sm">Runtime</AlertTitle>
            </div>
            <Badge variant={badgeVariant}>{badgeLabel}</Badge>
          </div>
          <AlertDescription className="space-y-1 text-sm">
            <p>{description}</p>
            <p className="text-xs text-muted-foreground">
              {state.loading ? 'Pending response…' : `Updated ${timestamp}`}
            </p>
          </AlertDescription>
        </Alert>
        <div className="flex items-center justify-end">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => {
              actions.refresh().catch(() => false);
            }}
            disabled={state.loading}
          >
            <RefreshCw className={state.loading ? 'mr-2 h-4 w-4 animate-spin' : 'mr-2 h-4 w-4'} />
            Refresh
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
