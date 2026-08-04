import { useEffect, useState } from 'react';
import type { ReactElement } from 'react';
import { getDiagnostics, onDiagnostic, type Unlisten } from '../../lib/ipc';
import type { DiagnosticEventDto } from '../../lib/ipc/types';
import { strings } from '../../lib/i18n/strings';

/// How many entries the view keeps. The native ring is also bounded (ADR-0005); this caps the DOM so a
/// long session cannot grow the view without limit.
export const VIEW_LIMIT = 20;

/// Formats the safe detail fields. Only allowlisted values appear — never payload contents, a raw
/// device id, a file path, or serial bytes (ADR-0005; the DTO has no field that could carry them).
function detail(event: DiagnosticEventDto): string {
  const t = strings.diagnostics;
  const parts: string[] = [];
  if (event.messageType) parts.push(event.messageType);
  if (event.seq !== null) parts.push(`${t.seq} ${event.seq}`);
  if (event.payloadLen !== null) parts.push(`${event.payloadLen} ${t.payloadLen}`);
  if (event.retryCount > 0) parts.push(`${t.retry} ${event.retryCount}`);
  if (event.deviceIdHashShort) parts.push(`${t.device} ${event.deviceIdHashShort}`);
  return parts.join(' · ');
}

/// The local, redacted diagnostics view (FR-031). Seeds from `get_diagnostics` then follows
/// `diagnostics://event`, keeping only the most recent [`VIEW_LIMIT`] entries.
export function Diagnostics(): ReactElement {
  const [events, setEvents] = useState<DiagnosticEventDto[]>([]);

  useEffect(() => {
    let active = true;
    let unlisten: Unlisten = () => {};
    void getDiagnostics(VIEW_LIMIT).then((initial) => {
      if (active) setEvents(initial.slice(-VIEW_LIMIT));
    });
    void onDiagnostic((event) => {
      if (active) setEvents((prev) => [...prev, event].slice(-VIEW_LIMIT));
    }).then((handle) => {
      if (active) unlisten = handle;
      else handle();
    });
    return () => {
      active = false;
      unlisten();
    };
  }, []);

  const t = strings.diagnostics;
  return (
    <section aria-labelledby="diag-heading" className="diagnostics">
      <h2 id="diag-heading">{t.heading}</h2>
      {events.length === 0 ? (
        <p className="diag-empty">{t.empty}</p>
      ) : (
        <div role="log" aria-live="polite" aria-label={t.live} className="diag-log">
          <ul className="diag-list">
            {events.map((event, index) => (
              <li
                key={`${event.at}-${index}`}
                className="diag-entry"
                data-category={event.category}
              >
                <time dateTime={event.at}>{event.at}</time>
                <span className="diag-category">{event.category}</span>
                <span className="diag-connection">{event.connection}</span>
                <span className="diag-detail">{detail(event)}</span>
              </li>
            ))}
          </ul>
        </div>
      )}
      <p className="diag-note">{t.note}</p>
    </section>
  );
}
