import { describe, expect, it, vi } from 'vitest';
import { blitRgba } from '../blit';

describe('blitRgba', () => {
  it('blits RGBA bytes to the canvas unmodified (constraint 2)', () => {
    const dim = 2;
    // prettier-ignore
    const rgba = new Uint8ClampedArray([
      255, 0, 0, 255,   0, 255, 0, 255,
      0, 0, 255, 255,   255, 255, 255, 255,
    ]);
    const putImageData = vi.fn();
    const ctx = { putImageData } as unknown as CanvasRenderingContext2D;

    blitRgba(ctx, rgba, dim);

    expect(putImageData).toHaveBeenCalledTimes(1);
    const image = putImageData.mock.calls[0][0] as ImageData;
    expect(Array.from(image.data)).toEqual(Array.from(rgba));
    expect(image.width).toBe(dim);
    expect(image.height).toBe(dim);
    // Blitted at the origin — no offset, scaling, or transform.
    expect(putImageData.mock.calls[0][1]).toBe(0);
    expect(putImageData.mock.calls[0][2]).toBe(0);
  });
});
