import { useEffect, useRef, useState } from 'react';
import type { ReactElement } from 'react';
import { Download, LoaderCircle } from 'lucide-react';
import { Button } from '../../components/ui/button';
import { flashFirmware, getFirmwareStatus } from '../../lib/ipc';
import type { FirmwareStatusDto } from '../../lib/ipc/types';
import { strings } from '../../lib/i18n/strings';

/** Native state keeps an update observable when Overview is left and reopened. */
export function FirmwareUpdate({ connected }: { connected: boolean }): ReactElement {
  const [status, setStatus] = useState<FirmwareStatusDto | null>(null);
  const [requesting, setRequesting] = useState(false);
  const [requestError, setRequestError] = useState<string | null>(null);
  const [pollError, setPollError] = useState<string | null>(null);
  const pending = useRef(false);
  const mounted = useRef(false);
  const t = strings.firmwareUpdate;

  useEffect(() => {
    mounted.current = true;
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async (): Promise<void> => {
      try {
        const snapshot = await getFirmwareStatus();
        if (active) {
          setStatus(snapshot);
          setPollError(null);
        }
      } catch (error) {
        if (active) setPollError(error instanceof Error ? error.message : String(error));
      } finally {
        if (active) timer = setTimeout(() => void poll(), 750);
      }
    };
    void poll();
    return () => {
      mounted.current = false;
      active = false;
      clearTimeout(timer);
    };
  }, []);

  const busy =
    requesting ||
    status?.phase === 'preparing' ||
    status?.phase === 'flashing' ||
    status?.phase === 'reconnecting';
  const canFlash = connected && status?.available && !busy && !pollError;
  const error = requestError ?? pollError;

  const start = async (): Promise<void> => {
    if (!canFlash || pending.current) return;
    pending.current = true;
    setRequesting(true);
    setRequestError(null);
    try {
      await flashFirmware();
      const snapshot = await getFirmwareStatus();
      if (mounted.current) setStatus(snapshot);
    } catch (failure) {
      if (mounted.current) {
        setRequestError(failure instanceof Error ? failure.message : String(failure || t.failed));
      }
    } finally {
      pending.current = false;
      if (mounted.current) setRequesting(false);
    }
  };

  return (
    <section aria-labelledby="firmware-heading" className="space-y-3">
      <div>
        <h3 id="firmware-heading" className="font-medium">
          {t.heading}
        </h3>
        <p className="mt-1 text-sm text-muted-foreground">{t.description}</p>
      </div>
      <Button disabled={!canFlash} onClick={() => void start()} aria-describedby="firmware-caution">
        {busy ? (
          <LoaderCircle aria-hidden="true" className="animate-spin" />
        ) : (
          <Download aria-hidden="true" />
        )}
        {busy ? t.working : t.action}
      </Button>
      <p id="firmware-caution" className="text-xs text-muted-foreground">
        {t.caution}
      </p>
      {!connected && !busy ? <p className="text-sm text-muted-foreground">{t.connect}</p> : null}
      <p
        aria-live="polite"
        className={
          status?.phase === 'failed' ? 'text-sm text-destructive' : 'text-sm text-muted-foreground'
        }
      >
        {requesting ? t.preparing : (status?.message ?? t.checking)}
      </p>
      {error ? (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      ) : null}
    </section>
  );
}
