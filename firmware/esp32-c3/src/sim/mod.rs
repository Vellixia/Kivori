//! Host simulation adapters (`host-sim` feature): an in-memory transport, a capture display, and a
//! virtual clock. Together they let the entire device core run and be asserted on the host with no
//! hardware (FR-035).

use crate::ports::{Clock, DisplaySink, Transport};
use core::cell::Cell;
use core::convert::Infallible;
use heapless::Vec;
use kivori_model::{ElapsedMs, Rect, Rgb565};

/// Byte capacity of each direction of the simulated pipe.
pub const PIPE_CAPACITY: usize = 8192;
/// Simulated panel width.
pub const FRAME_W: usize = 240;
/// Simulated panel height.
pub const FRAME_H: usize = 240;
/// Simulated panel pixel count.
pub const FRAME_PIXELS: usize = FRAME_W * FRAME_H;

/// A bidirectional in-memory byte pipe. The device sees it as a [`Transport`]; the test plays the
/// host role via [`SimPipe::host_send`] / [`SimPipe::host_recv`].
pub struct SimPipe {
    to_device: Vec<u8, PIPE_CAPACITY>,
    to_host: Vec<u8, PIPE_CAPACITY>,
}

impl SimPipe {
    /// An empty pipe.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            to_device: Vec::new(),
            to_host: Vec::new(),
        }
    }

    /// Host → device: queues `bytes` for the device to read. `Err` if the pipe is full.
    // Mirrors heapless `extend_from_slice`'s own `Result<(), ()>`; a sim helper needs no richer error.
    #[allow(clippy::result_unit_err)]
    pub fn host_send(&mut self, bytes: &[u8]) -> Result<(), ()> {
        self.to_device.extend_from_slice(bytes)
    }

    /// Device → host: drains and returns everything the device has written so far.
    #[must_use]
    pub fn host_recv(&mut self) -> Vec<u8, PIPE_CAPACITY> {
        let out = self.to_host.clone();
        self.to_host.clear();
        out
    }

    /// Whether the device has produced any unread output.
    #[must_use]
    pub fn host_has_output(&self) -> bool {
        !self.to_host.is_empty()
    }
}

impl Default for SimPipe {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for SimPipe {
    type Error = Infallible;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let n = buf.len().min(self.to_device.len());
        buf[..n].copy_from_slice(&self.to_device[..n]);
        let mut rest: Vec<u8, PIPE_CAPACITY> = Vec::new();
        let _ = rest.extend_from_slice(&self.to_device[n..]);
        self.to_device = rest;
        Ok(n)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let space = PIPE_CAPACITY - self.to_host.len();
        let n = space.min(buf.len());
        let _ = self.to_host.extend_from_slice(&buf[..n]);
        Ok(n)
    }
}

/// A [`DisplaySink`] that composites blitted tiles into a full-frame buffer and counts flushes.
pub struct CaptureDisplay {
    frame: [Rgb565; FRAME_PIXELS],
    /// Number of tile blits received (change-driven rendering flushes only changed tiles).
    pub blits: u32,
}

impl CaptureDisplay {
    /// A capture display cleared to black.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            frame: [Rgb565::from_raw(0); FRAME_PIXELS],
            blits: 0,
        }
    }

    /// The full composited frame (row-major, `FRAME_W * FRAME_H`).
    #[must_use]
    pub fn frame(&self) -> &[Rgb565] {
        &self.frame
    }

    /// The pixel at `(x, y)`.
    #[must_use]
    pub fn pixel(&self, x: usize, y: usize) -> Rgb565 {
        self.frame[y * FRAME_W + x]
    }
}

impl Default for CaptureDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplaySink for CaptureDisplay {
    type Error = Infallible;

    fn blit_tile(&mut self, rect: Rect, pixels: &[Rgb565]) -> Result<(), Self::Error> {
        let (x0, y0) = (rect.x as usize, rect.y as usize);
        let (w, h) = (rect.w as usize, rect.h as usize);
        for row in 0..h {
            for col in 0..w {
                let (dx, dy) = (x0 + col, y0 + row);
                if dx < FRAME_W && dy < FRAME_H {
                    self.frame[dy * FRAME_W + dx] = pixels[row * w + col];
                }
            }
        }
        self.blits += 1;
        Ok(())
    }
}

/// A settable, monotonic virtual [`Clock`].
pub struct VirtualClock {
    now: Cell<ElapsedMs>,
}

impl VirtualClock {
    /// A clock at `0` ms.
    #[must_use]
    pub const fn new() -> Self {
        Self { now: Cell::new(0) }
    }

    /// Sets the current time.
    pub fn set(&self, ms: ElapsedMs) {
        self.now.set(ms);
    }

    /// Advances the clock by `delta` ms (saturating).
    pub fn advance(&self, delta: ElapsedMs) {
        self.now.set(self.now.get().saturating_add(delta));
    }
}

impl Default for VirtualClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for VirtualClock {
    fn now_ms(&self) -> ElapsedMs {
        self.now.get()
    }
}
