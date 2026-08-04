import { useEffect, useRef } from 'react';
import type { ReactElement } from 'react';
import { renderPreviewFrame } from '../ipc';
import type { CompanionState } from '../ipc/types';
import { PREVIEW_DIM } from '../ipc/types';
import { blitRgba } from './blit';

interface DevicePreviewProps {
  state: CompanionState;
  elapsedMs: number;
  /** Accessible label for the preview image (screen readers announce the pixels' meaning). */
  label: string;
  /**
   * A frame pushed from the native preview stream. When present it is blitted as-is and no per-frame
   * request is made; when absent the exact frame for `(state, elapsedMs)` is fetched (scrub/step).
   * Either way the bytes come from the shared Rust renderer — the canvas never draws content itself.
   */
  frame?: Uint8ClampedArray | null;
}

/** Blits the current preview frame onto a blit-only canvas. */
export function DevicePreview({
  state,
  elapsedMs,
  label,
  frame = null,
}: DevicePreviewProps): ReactElement {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  // Streamed path: blit whatever the native producer pushed.
  useEffect(() => {
    if (!frame) return;
    const ctx = canvasRef.current?.getContext('2d');
    if (ctx) blitRgba(ctx, frame, PREVIEW_DIM);
  }, [frame]);

  // On-demand path: request the exact frame for this (state, elapsedMs).
  useEffect(() => {
    if (frame) return;
    let cancelled = false;
    void renderPreviewFrame(state, elapsedMs).then((rgba) => {
      if (cancelled) return;
      const ctx = canvasRef.current?.getContext('2d');
      if (ctx) blitRgba(ctx, rgba, PREVIEW_DIM);
    });
    return () => {
      cancelled = true;
    };
  }, [state, elapsedMs, frame]);

  return (
    <canvas
      ref={canvasRef}
      width={PREVIEW_DIM}
      height={PREVIEW_DIM}
      role="img"
      aria-label={label}
      className="device-preview"
    />
  );
}
