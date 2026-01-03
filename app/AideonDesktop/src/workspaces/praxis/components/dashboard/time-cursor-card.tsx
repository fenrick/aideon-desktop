import { useTemporalPanel } from 'praxis/time/use-temporal-panel';

import { TimeControlPanel } from 'praxis/components/blocks/time-control-panel';

/**
 *
 */
export function TimeCursorCard() {
  const [state, actions] = useTemporalPanel();
  return <TimeControlPanel state={state} actions={actions} />;
}
