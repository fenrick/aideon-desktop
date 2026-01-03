import { CheckCircle2 } from 'lucide-react';

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from 'design-system/components/ui/card';

const PHASES = [
  { title: 'Bootstrap shell', status: 'complete' },
  { title: 'praxisApi IPC', status: 'complete' },
  { title: 'React Flow graph widget', status: 'complete' },
  { title: 'Catalogue + Matrix widgets', status: 'complete' },
  { title: 'Charts + templates', status: 'in-progress' },
];

/**
 *
 */
export function PhaseCheckpointsCard() {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Phase checkpoints</CardTitle>
        <CardDescription>Tracked milestones for the React canvas build.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        {PHASES.map((phase) => (
          <div key={phase.title} className="flex items-center gap-3">
            <span
              className="flex h-8 w-8 items-center justify-center rounded-full border border-border/70 bg-muted/30"
              aria-hidden
            >
              <CheckCircle2 className={statusIconClass(phase.status)} />
            </span>
            <div>
              <p className="text-sm font-medium text-foreground">{phase.title}</p>
              <p className="text-xs text-muted-foreground capitalize">{phase.status}</p>
            </div>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}

/**
 *
 * @param status
 */
function statusIconClass(status: string): string {
  if (status === 'complete') {
    return 'h-4 w-4 text-primary';
  }
  if (status === 'in-progress') {
    return 'h-4 w-4 text-primary/70';
  }
  return 'h-4 w-4 text-muted-foreground';
}
