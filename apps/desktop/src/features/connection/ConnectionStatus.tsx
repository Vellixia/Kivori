import { useEffect, useState } from 'react';
import type { ReactElement } from 'react';
import {
  CheckCircle2,
  LoaderCircle,
  TriangleAlert,
  Unplug,
  type LucideIcon,
} from 'lucide-react';
import { Badge } from '../../components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '../../components/ui/card';
import { Separator } from '../../components/ui/separator';
import { getConnectionStatus, onConnectionStatus, type Unlisten } from '../../lib/ipc';
import type { ConnectionStatusDto } from '../../lib/ipc/types';
import { strings } from '../../lib/i18n/strings';

/// The six UI states the user sees (derived from the connection axis + retry count).
type UiStatus =
  | 'disconnected'
  | 'connecting'
  | 'connected'
  | 'reconnecting'
  | 'incompatible'
  | 'error';

const STATUS_ICONS: Record<UiStatus, LucideIcon> = {
  disconnected: Unplug,
  connecting: LoaderCircle,
  connected: CheckCircle2,
  reconnecting: LoaderCircle,
  incompatible: TriangleAlert,
  error: TriangleAlert,
};

/// Maps the raw connection status to a UI status: an error auto-retries, and a disconnected state with
/// prior attempts reads as "reconnecting" (FR-008).
function uiStatus(status: ConnectionStatusDto): UiStatus {
  switch (status.connection) {
    case 'connected':
      return 'connected';
    case 'connecting':
      return 'connecting';
    case 'incompatible':
      return 'incompatible';
    case 'error':
      return 'reconnecting';
    case 'disconnected':
      return status.retryCount > 0 ? 'reconnecting' : 'disconnected';
    default:
      return 'disconnected';
  }
}

/// The production connection-status surface (FR-005/006). Subscribes to `connection://status` and
/// seeds from the initial snapshot so correctness never depends on subscription timing.
export function ConnectionStatus(): ReactElement {
  const [status, setStatus] = useState<ConnectionStatusDto | null>(null);

  useEffect(() => {
    let active = true;
    let unlisten: Unlisten = () => {};
    void getConnectionStatus().then((snapshot) => {
      if (active) setStatus(snapshot);
    });
    void onConnectionStatus((snapshot) => {
      if (active) setStatus(snapshot);
    }).then((handle) => {
      if (active) unlisten = handle;
      else handle();
    });
    return () => {
      active = false;
      unlisten();
    };
  }, []);

  const t = strings.connection;
  const ui = status ? uiStatus(status) : 'disconnected';
  const StatusIcon = STATUS_ICONS[ui];
  const badgeVariant = ui === 'connected' ? 'default' : ui === 'incompatible' ? 'destructive' : 'secondary';

  return (
    <section aria-labelledby="conn-heading" className="connection-status">
      <Card>
        <CardHeader>
          <CardTitle id="conn-heading">{t.heading}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <Badge variant={badgeVariant} data-state={ui} role="status" aria-live="polite">
              <StatusIcon data-icon="inline-start" aria-hidden="true" />
              {t.status[ui]}
              {status && status.retryCount > 0 ? ` (${t.attempt} ${status.retryCount})` : ''}
            </Badge>
            {status?.incompatibleReason ? (
              <p className="conn-reason mt-2 text-sm text-destructive">{status.incompatibleReason}</p>
            ) : null}
          </div>

          {status?.device ? (
            <dl className="conn-device grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
              <dt className="text-muted-foreground">{t.firmware}</dt>
              <dd>{status.device.firmwareVersion}</dd>
              <dt className="text-muted-foreground">{t.protocol}</dt>
              <dd>{`${status.device.protocolVersion.major}.${status.device.protocolVersion.minor}`}</dd>
              <dt className="text-muted-foreground">{t.deviceId}</dt>
              <dd className="font-mono">{status.device.deviceIdHashShort}</dd>
            </dl>
          ) : null}

          <Separator />

          <dl className="conn-axes grid grid-cols-2 gap-3 sm:grid-cols-4">
            <div>
              <dt className="text-xs text-muted-foreground">{t.desired}</dt>
              <dd className="mt-1 font-medium" data-testid="desired">
                {status?.desired ?? '—'}
              </dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">{t.reported}</dt>
              <dd className="mt-1 font-medium" data-testid="reported">
                {status?.reported ?? '—'}
              </dd>
            </div>
          </dl>
        </CardContent>
      </Card>
    </section>
  );
}
