import { useEffect, useState } from 'react';
import type { ReactElement } from 'react';
import { ConnectionStatus } from './features/connection/ConnectionStatus';
import { Diagnostics } from './features/connection/Diagnostics';
import { DeviceStudio } from './features/device-studio/DeviceStudio';
import { getAppInfo } from './lib/ipc';

// The Device Studio route is served in dev builds only (FR-028). The backend independently gates its
// dev-only IPC commands behind the `device-studio` Cargo feature, so both layers must agree.
const DEVICE_STUDIO_BUILD = import.meta.env.DEV;

export function App(): ReactElement {
  const [studioEnabled, setStudioEnabled] = useState(false);

  useEffect(() => {
    let active = true;
    void getAppInfo().then((info) => {
      if (active) setStudioEnabled(info.deviceStudioEnabled);
    });
    return () => {
      active = false;
    };
  }, []);

  return (
    <main>
      <ConnectionStatus />
      <Diagnostics />
      {DEVICE_STUDIO_BUILD && studioEnabled ? <DeviceStudio /> : null}
    </main>
  );
}
