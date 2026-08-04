import { useEffect, useState } from 'react';
import type { ReactElement } from 'react';
import { getConnectionStatus, onConnectionStatus, type Unlisten } from '../../lib/ipc';
import type { ConnectionStatusDto } from '../../lib/ipc/types';
import { strings } from '../../lib/i18n/strings';

/// The six UI states the user sees (derived from the connection axis + retry count).
type UiStatus =
  'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'incompatible' | 'error';

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

  return (
    <section aria-labelledby="conn-heading" className="connection-status">
      <h1 id="conn-heading">{strings.appTitle}</h1>
      <p role="status" aria-live="polite" data-state={ui} className="conn-line">
        <span className="conn-dot" data-state={ui} aria-hidden="true" />
        {t.status[ui]}
        {status && status.retryCount > 0 ? ` (${t.attempt} ${status.retryCount})` : ''}
      </p>
      {status?.incompatibleReason ? (
        <p className="conn-reason">{status.incompatibleReason}</p>
      ) : null}
      {status?.device ? (
        <dl className="conn-device">
          <dt>{t.firmware}</dt>
          <dd>{status.device.firmwareVersion}</dd>
          <dt>{t.protocol}</dt>
          <dd>{`${status.device.protocolVersion.major}.${status.device.protocolVersion.minor}`}</dd>
          <dt>{t.deviceId}</dt>
          <dd>{status.device.deviceIdHashShort}</dd>
        </dl>
      ) : null}
      <dl className="conn-axes">
        <dt>{t.desired}</dt>
        <dd data-testid="desired">{status?.desired ?? '—'}</dd>
        <dt>{t.reported}</dt>
        <dd data-testid="reported">{status?.reported ?? '—'}</dd>
      </dl>
    </section>
  );
}
