import { ActivityTimelinePanel } from 'praxis/components/blocks/activity-timeline-panel';
import { useTemporalPanel } from 'praxis/time/use-temporal-panel';

/**
 * Wrapper card displaying the activity timeline panel.
 * @returns Activity feed component.
 */
export function ActivityFeedCard() {
  const [state, actions] = useTemporalPanel();
  return <ActivityTimelinePanel state={state} actions={actions} />;
}
