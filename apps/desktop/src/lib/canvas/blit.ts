// Canvas blit surface (contracts/ipc.md constraint 2): the canvas ONLY blits pre-rendered RGBA bytes.
// No drawing, scaling, or color logic lives here — the shared Rust renderer is the single source of
// truth for pixels, so the frontend can never diverge from what the device shows.

/** Blits a row-major RGBA8888 `dim`×`dim` buffer to the top-left of `ctx`, byte-for-byte. */
export function blitRgba(
  ctx: CanvasRenderingContext2D,
  rgba: Uint8ClampedArray,
  dim: number,
): void {
  // Copy into a fresh, ArrayBuffer-backed array: `ImageData` requires an `ArrayBuffer` (not a
  // `SharedArrayBuffer`), and the incoming view's backing store is not statically known.
  const data = new Uint8ClampedArray(rgba);
  ctx.putImageData(new ImageData(data, dim, dim), 0, 0);
}
