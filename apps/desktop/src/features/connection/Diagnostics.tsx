import { useEffect, useState } from 'react';
import type { ReactElement } from 'react';
import { Badge } from '../../components/ui/badge';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../../components/ui/card';
import { ScrollArea } from '../../components/ui/scroll-area';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../../components/ui/table';
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
      <Card>
        <CardHeader>
          <CardTitle id="diag-heading">{t.heading}</CardTitle>
          <CardDescription>{t.note}</CardDescription>
        </CardHeader>
        <CardContent>
          {events.length === 0 ? (
            <p className="diag-empty text-sm text-muted-foreground">{t.empty}</p>
          ) : (
            <ScrollArea className="max-h-96">
              <div role="log" aria-live="polite" aria-label={t.live} className="diag-log">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{t.columns.at}</TableHead>
                      <TableHead>{t.columns.category}</TableHead>
                      <TableHead>{t.columns.connection}</TableHead>
                      <TableHead>{t.columns.detail}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {events.map((event, index) => (
                      <TableRow key={`${event.at}-${index}`} data-category={event.category}>
                        <TableCell>
                          <time dateTime={event.at}>{event.at}</time>
                        </TableCell>
                        <TableCell>
                          <Badge variant="outline">{event.category}</Badge>
                        </TableCell>
                        <TableCell>
                          <Badge variant="secondary">{event.connection}</Badge>
                        </TableCell>
                        <TableCell className="whitespace-normal text-muted-foreground">
                          {detail(event)}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            </ScrollArea>
          )}
        </CardContent>
      </Card>
    </section>
  );
}
