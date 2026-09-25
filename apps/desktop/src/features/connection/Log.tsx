import { useEffect, useMemo, useState } from 'react';
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
import { getActivityLog, onActivityLog, type Unlisten } from '../../lib/ipc';
import type { ActivityEventDto, ActivitySeverity, ActivitySource } from '../../lib/ipc/types';
import { strings } from '../../lib/i18n/strings';

/** The native session view is deliberately bounded to its 256 newest event IDs. */
export const VIEW_LIMIT = 256;

type Filter<T extends string> = 'all' | T;
type LogStatus = 'starting' | 'live' | 'unavailable';

function mergeEvents(
  history: readonly ActivityEventDto[],
  live: readonly ActivityEventDto[],
): ActivityEventDto[] {
  const byId = new Map<number, ActivityEventDto>();
  for (const event of history) byId.set(event.id, event);
  for (const event of live) byId.set(event.id, event);
  return [...byId.values()].sort((a, b) => a.id - b.id).slice(-VIEW_LIMIT);
}

/** Formats only fixed metadata properties; unrecognised runtime fields cannot reach the UI. */
function details(event: ActivityEventDto): string {
  const metadata = event.metadata;
  if (!metadata) return '';
  const values: string[] = [];
  if (metadata.connection) values.push(`connection ${metadata.connection}`);
  if (metadata.retryCount > 0) values.push(`retry ${metadata.retryCount}`);
  if (metadata.elapsedMs > 0) values.push(`elapsed ${metadata.elapsedMs} ms`);
  if (metadata.diagnosticCategory) values.push(`diagnostic ${metadata.diagnosticCategory}`);
  if (metadata.diagnosticCode !== undefined) values.push(`code ${metadata.diagnosticCode}`);
  if (metadata.firmwareVersion) values.push(`firmware ${metadata.firmwareVersion}`);
  if (metadata.protocolVersion)
    values.push(`protocol ${metadata.protocolVersion.major}.${metadata.protocolVersion.minor}`);
  if (metadata.deviceIdHashShort) values.push(`device ${metadata.deviceIdHashShort}`);
  if (metadata.capabilities !== undefined) values.push(`capabilities ${metadata.capabilities}`);
  if (metadata.state) values.push(`state ${metadata.state}`);
  if (metadata.personality) values.push(`personality ${metadata.personality}`);
  if (metadata.selfPlay !== undefined) values.push(`self-play ${metadata.selfPlay ? 'on' : 'off'}`);
  if (metadata.action) values.push(`action ${metadata.action}`);
  if (metadata.seed !== undefined) values.push(`seed ${metadata.seed}`);
  if (metadata.appliedAtMs !== undefined) values.push(`applied ${metadata.appliedAtMs} ms`);
  if (metadata.autonomous !== undefined)
    values.push(`autonomous ${metadata.autonomous ? 'yes' : 'no'}`);
  if (metadata.protocolCategory) values.push(`protocol ${metadata.protocolCategory}`);
  if (metadata.payloadLen !== undefined) values.push(`bytes ${metadata.payloadLen}`);
  if (metadata.sequence !== undefined) values.push(`sequence ${metadata.sequence}`);
  if (metadata.skipped !== undefined) values.push(`skipped ${metadata.skipped}`);
  if (metadata.reported) values.push(`reported ${metadata.reported}`);
  return values.join(' · ');
}

export function Log(): ReactElement {
  const [events, setEvents] = useState<ActivityEventDto[]>([]);
  const [severity, setSeverity] = useState<Filter<ActivitySeverity>>('all');
  const [source, setSource] = useState<Filter<ActivitySource>>('all');
  const [status, setStatus] = useState<LogStatus>('starting');

  useEffect(() => {
    let active = true;
    let unlisten: Unlisten | undefined;
    const setup = async (): Promise<void> => {
      try {
        const handle = await onActivityLog((event) => {
          if (active) setEvents((current) => mergeEvents(current, [event]));
        });
        if (!active) {
          handle();
          return;
        }
        unlisten = handle;
        setStatus('live');
        const history = await getActivityLog(VIEW_LIMIT);
        if (active) setEvents((current) => mergeEvents(history, current));
      } catch {
        if (active) setStatus('unavailable');
      }
    };
    void setup();
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  const filtered = useMemo(
    () =>
      events.filter(
        (event) =>
          (severity === 'all' || event.severity === severity) &&
          (source === 'all' || event.source === source),
      ),
    [events, severity, source],
  );
  const t = strings.log;

  return (
    <section aria-labelledby="log-heading" className="activity-log">
      <Card>
        <CardHeader>
          <CardTitle id="log-heading">{t.heading}</CardTitle>
          <CardDescription>{t.note}</CardDescription>
        </CardHeader>
        <CardContent>
          <p role="status" aria-live="polite" className="mb-4 text-sm text-muted-foreground">
            {t.status[status]}
          </p>
          <div className="mb-4 flex flex-wrap gap-4" aria-label={t.filters.label}>
            <label className="flex items-center gap-2 text-sm" htmlFor="log-severity">
              {t.filters.severity}
              <select
                id="log-severity"
                value={severity}
                onChange={(event) => setSeverity(event.target.value as Filter<ActivitySeverity>)}
              >
                <option value="all">{t.filters.all}</option>
                <option value="info">info</option>
                <option value="warning">warning</option>
                <option value="error">error</option>
              </select>
            </label>
            <label className="flex items-center gap-2 text-sm" htmlFor="log-source">
              {t.filters.source}
              <select
                id="log-source"
                value={source}
                onChange={(event) => setSource(event.target.value as Filter<ActivitySource>)}
              >
                <option value="all">{t.filters.all}</option>
                <option value="connection">connection</option>
                <option value="action">action</option>
                <option value="device">device</option>
                <option value="protocol">protocol</option>
                <option value="firmware">firmware</option>
              </select>
            </label>
          </div>
          {events.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t.empty}</p>
          ) : filtered.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t.noMatches}</p>
          ) : (
            <ScrollArea className="max-h-96">
              <div role="log" aria-live="polite" aria-label={t.live}>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{t.columns.time}</TableHead>
                      <TableHead>{t.columns.severity}</TableHead>
                      <TableHead>{t.columns.source}</TableHead>
                      <TableHead>{t.columns.event}</TableHead>
                      <TableHead>{t.columns.result}</TableHead>
                      <TableHead>{t.columns.details}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filtered.map((event) => (
                      <TableRow key={event.id} data-event-id={event.id}>
                        <TableCell>
                          <time dateTime={event.at}>{event.at}</time>
                        </TableCell>
                        <TableCell>
                          <Badge variant="outline">{event.severity}</Badge>
                        </TableCell>
                        <TableCell>
                          <Badge variant="secondary">{event.source}</Badge>
                        </TableCell>
                        <TableCell className="whitespace-normal">
                          {event.type}: {event.summary}
                        </TableCell>
                        <TableCell>{event.outcome}</TableCell>
                        <TableCell className="whitespace-normal text-muted-foreground">
                          {details(event)}
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
