import '@testing-library/jest-dom/vitest';

// jsdom has no canvas backend and (in some versions) no `ImageData` constructor. The blit path only
// needs `ImageData` to carry bytes, so provide a minimal polyfill — this keeps canvas tests free of a
// native `node-canvas` dependency while still exercising the real blit code path.
if (typeof globalThis.ImageData === 'undefined') {
  class ImageDataPolyfill {
    readonly data: Uint8ClampedArray;
    readonly width: number;
    readonly height: number;
    readonly colorSpace: PredefinedColorSpace = 'srgb';
    constructor(data: Uint8ClampedArray, width: number, height: number) {
      this.data = data;
      this.width = width;
      this.height = height;
    }
  }
  globalThis.ImageData = ImageDataPolyfill as unknown as typeof ImageData;
}
