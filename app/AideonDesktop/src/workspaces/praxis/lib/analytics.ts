export type AnalyticsEventName =
  | 'template.change'
  | 'template.create_widget'
  | 'selection.change'
  | 'time.cursor'
  | 'inspector.save'
  | 'error.ui';

export type AnalyticsSink = (event: AnalyticsEventName, payload?: Record<string, unknown>) => void;

const ringBuffer: { event: AnalyticsEventName; payload?: Record<string, unknown>; at: number }[] =
  [];
const MAX_EVENTS = 50;

let sink: AnalyticsSink = (_event, _payload) => false;

/**
 *
 * @param nextSink
 */
export function setAnalyticsSink(nextSink: AnalyticsSink) {
  sink = nextSink;
}

/**
 *
 * @param event
 * @param payload
 */
export function track(event: AnalyticsEventName, payload?: Record<string, unknown>) {
  sink(event, sanitize(payload));
  ringBuffer.unshift({ event, payload: sanitize(payload), at: Date.now() });
  if (ringBuffer.length > MAX_EVENTS) {
    ringBuffer.pop();
  }
}

/**
 *
 */
export function recentAnalytics(): readonly {
  event: AnalyticsEventName;
  payload?: Record<string, unknown>;
  at: number;
}[] {
  return ringBuffer;
}

/**
 *
 * @param payload
 */
function sanitize(payload?: Record<string, unknown>) {
  if (!payload) {
    return;
  }
  const entries = Object.entries(payload).filter(([_, value]) => {
    return typeof value !== 'function';
  });
  return Object.fromEntries(entries);
}
