import { useEffect, useRef, useState } from 'react';
import type { ReactElement } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '../../components/ui/card';
import { DevicePreview } from '../../lib/canvas/DevicePreview';
import { openPreviewStream, type PreviewStream } from '../../lib/ipc';
import { PREVIEW_FPS } from '../../lib/ipc/types';
import { strings } from '../../lib/i18n/strings';
import { Controls } from './Controls';
import { useStudioStore } from './store';

/**
 * Dev-only Device Studio: live preview of the shared renderer plus playback controls (US4).
 *
 * Playback uses the native preview-frame stream (contracts/ipc.md §3): the Rust producer renders at a
 * capped FPS and pushes RGBA bytes, which the canvas blits. Paused scrub/step falls back to a per-frame
 * `render_preview_frame` request, which returns the exact frame for an exact millisecond (SC-011).
 */
export function DeviceStudio(): ReactElement {
  const state = useStudioStore((s) => s.state);
  const elapsedMs = useStudioStore((s) => s.elapsedMs);
  const playing = useStudioStore((s) => s.playing);
  const advance = useStudioStore((s) => s.advance);
  const rafRef = useRef<number | null>(null);
  const [streamFrame, setStreamFrame] = useState<Uint8ClampedArray | null>(null);

  // Advance the timeline while playing (drift-free integer stepping lives in the store).
  useEffect(() => {
    if (!playing) return;
    let last = performance.now();
    const tick = (now: number): void => {
      advance(now - last);
      last = now;
      rafRef.current = requestAnimationFrame(tick);
    };
    rafRef.current = requestAnimationFrame(tick);
    return () => {
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
    };
  }, [playing, advance]);

  // Open a native frame stream while playing; close it on pause, state change, or unmount.
  useEffect(() => {
    if (!playing) {
      setStreamFrame(null);
      return;
    }
    let active = true;
    let stream: PreviewStream | null = null;
    void openPreviewStream(state, PREVIEW_FPS, (frame) => {
      if (active) setStreamFrame(frame);
    }).then((opened) => {
      if (active) stream = opened;
      else void opened.close();
    });
    return () => {
      active = false;
      setStreamFrame(null);
      void stream?.close();
    };
  }, [playing, state]);

  const t = strings.studio;
  return (
    <section aria-labelledby="studio-heading" className="device-studio space-y-4">
      <h2 id="studio-heading" className="text-xl font-semibold tracking-tight">
        {t.heading}
      </h2>
      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(20rem,0.8fr)]">
        <Card>
          <CardHeader>
            <CardTitle>{t.preview}</CardTitle>
          </CardHeader>
          <CardContent className="flex justify-center">
            <DevicePreview
              state={state}
              elapsedMs={elapsedMs}
              label={t.preview}
              frame={playing ? streamFrame : null}
            />
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>{t.controls}</CardTitle>
          </CardHeader>
          <CardContent>
            <Controls />
          </CardContent>
        </Card>
      </div>
    </section>
  );
}
