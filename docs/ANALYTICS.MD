# Analytics and Telemetry

**Principles:** privacy-first, no external endpoints by default. Events are emitted through an
injectable analytics interface so hosts can wire to their chosen sink.

## Event catalogue

- `template.change` - user switches template. Payload: `{ templateId, scenarioId }`.
- `template.create_widget` - widget added from registry. Payload: `{ widgetType, templateId }`.
- `selection.change` - widget/node/edge selection updated. Payload:
  `{ kind, sourceWidgetId, nodeCount, edgeCount }`.
- `time.cursor` - time context changed. Payload: `{ scenarioId, validTime, layer }`.
- `inspector.save` - property save dispatched. Payload: `{ selectionKind, selectionId }`.
- `error.ui` - user-visible error banner shown. Payload: `{ surface, message }`.

## Implementation hooks

- Analytics lives in `canvas/lib/analytics.ts`; default sink logs to `console.debug` in dev.
- Use `setAnalyticsSink(fn)` to inject a host implementation; keep payloads JSON-serialisable.
- Events must never include PII; redact free-form text beyond selection ids.

## Debugging

- Dev overlay (`Cmd/Ctrl+Shift+D`) shows current scenario, template, selection, and the most recent
  analytics events (in-memory ring buffer, non-persistent).
