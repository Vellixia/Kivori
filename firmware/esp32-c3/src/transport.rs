//! USB Serial/JTAG transport (T071).
//!
//! The real device transport: a **FIFO-aware, bounded, non-blocking** [`Transport`] over the ESP32-C3's
//! USB Serial/JTAG peripheral. The production run loop and the Wokwi external-serial mode both drive this
//! adapter, so the receive/transmit path under test is the shipping one, not a stand-in.
//!
//! # Why bounded and non-blocking matters
//!
//! The endpoint FIFO is 64 bytes ("Up to 64-byte data" per the SoC, and esp-hal's own `write` documents
//! "chunks of up to 64 bytes"). `esp_hal`'s `write` + `flush_tx` pair **blocks until the host drains the
//! FIFO**, which on a device whose host has stopped reading would stall the render loop indefinitely.
//! This adapter therefore uses the `_nb` primitives: it accepts at most one FIFO's worth per call, stops
//! the moment the hardware reports `WouldBlock`, and returns the number of bytes genuinely accepted — the
//! [`Transport`] contract. Receiving uses `drain_rx_fifo`, which empties the RX FIFO in one call instead of
//! byte-at-a-time polling.
//!
//! Because a single `write` can be short, [`TxBuffered`] wraps this adapter for the run loop: the
//! dispatcher writes whole frames into a queue that comfortably exceeds `MAX_WIRE`, and the loop pumps the
//! queue toward the hardware every tick. Without it, a full FIFO would truncate a frame mid-flight.
//!
//! Physical USB behaviour (host enumeration as `0x303A:0x1001`, real throughput, genuine transmit stalls
//! when the host stops draining) is NOT verified by simulation and remains a hardware task (T115/T116).

use crate::ports::Transport;
use esp_hal::peripherals::USB_DEVICE;
use esp_hal::usb_serial_jtag::{UsbSerialJtag, UsbSerialJtagRx, UsbSerialJtagTx};
use esp_hal::Blocking;
use heapless::Deque;
use kivori_protocol::MAX_WIRE;

/// Endpoint FIFO depth in bytes. One `write` call never offers the hardware more than this.
pub const FIFO_BYTES: usize = 64;

/// Outbound queue capacity: two full wire packets, so a whole frame always fits even with a partial
/// packet still draining.
pub const TX_QUEUE_BYTES: usize = MAX_WIRE * 2;

/// A [`Transport`] over USB Serial/JTAG.
///
/// The peripheral is split into its receive and transmit halves, because `drain_rx_fifo` — the FIFO-aware
/// bulk read — only exists on the receive half.
pub struct UsbJtagTransport<'d> {
    rx: UsbSerialJtagRx<'d, Blocking>,
    tx: UsbSerialJtagTx<'d, Blocking>,
}

impl<'d> UsbJtagTransport<'d> {
    /// Takes the USB Serial/JTAG peripheral and exposes it as a transport.
    pub fn new(usb_device: USB_DEVICE<'d>) -> Self {
        let (rx, tx) = UsbSerialJtag::new(usb_device).split();
        Self { rx, tx }
    }

    /// Writes raw bytes, **blocking** until the FIFO drains. Used only for the simulation harnesses' text
    /// markers, where a stalled marker is better than a lost one; protocol frames go through
    /// [`Transport::write`], which never blocks.
    pub fn write_all(&mut self, bytes: &[u8]) {
        let _ = self.tx.write(bytes);
        let _ = self.tx.flush_tx();
    }
}

impl Transport for UsbJtagTransport<'_> {
    /// Errors are reported as the unit type: the caller's recovery (drop the link, reconnect) does not
    /// depend on which USB error occurred, and no error detail may reach a log (ADR-0005).
    type Error = ();

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        // Drains whatever the RX FIFO holds right now and returns immediately when it is empty.
        let n = self.rx.drain_rx_fifo(buf);
        #[cfg(feature = "debug-payloads")]
        crate::debug_payloads::dump("rx", &buf[..n]);
        Ok(n)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        // Bounded: never offer the hardware more than one FIFO per call.
        let limit = buf.len().min(FIFO_BYTES);
        let mut accepted = 0;
        while accepted < limit {
            // Non-blocking: `WouldBlock` means the FIFO is full, so stop and report what was taken.
            if self.tx.write_byte_nb(buf[accepted]).is_err() {
                break;
            }
            accepted += 1;
        }
        // Best-effort, non-blocking flush: a busy FIFO must not stall the caller.
        let _ = self.tx.flush_tx_nb();
        #[cfg(feature = "debug-payloads")]
        crate::debug_payloads::dump("tx", &buf[..accepted]);
        Ok(accepted)
    }
}

/// A [`Transport`] wrapper giving the dispatcher whole-frame writes over a bounded hardware FIFO.
///
/// `write` enqueues (so a frame is never truncated by a full FIFO), `read` delegates straight through, and
/// [`Self::pump`] moves queued bytes toward the hardware without blocking. The run loop pumps every tick.
pub struct TxBuffered<T> {
    inner: T,
    queue: Deque<u8, TX_QUEUE_BYTES>,
}

impl<T: Transport> TxBuffered<T> {
    /// Wraps `inner` with an empty outbound queue.
    #[must_use]
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            queue: Deque::new(),
        }
    }

    /// Bytes still waiting to reach the hardware.
    #[must_use]
    pub fn pending(&self) -> usize {
        self.queue.len()
    }

    /// Borrows the wrapped transport (for adapter-specific calls such as blocking marker writes).
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Pushes as much of the queue to the hardware as it accepts right now. Returns bytes written.
    ///
    /// # Errors
    /// The wrapped transport's error, unchanged.
    pub fn pump(&mut self) -> Result<usize, T::Error> {
        let mut written = 0;
        while !self.queue.is_empty() {
            // Copy a contiguous chunk out of the ring, then only drop what the hardware took.
            let mut chunk = [0u8; FIFO_BYTES];
            let n = self.queue.len().min(FIFO_BYTES);
            for (slot, byte) in chunk[..n].iter_mut().zip(self.queue.iter()) {
                *slot = *byte;
            }
            let taken = self.inner.write(&chunk[..n])?;
            for _ in 0..taken {
                let _ = self.queue.pop_front();
            }
            written += taken;
            if taken < n {
                break; // hardware is full for now
            }
        }
        Ok(written)
    }
}

impl<T: Transport> Transport for TxBuffered<T> {
    type Error = T::Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.inner.read(buf)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let mut accepted = 0;
        for &byte in buf {
            if self.queue.push_back(byte).is_err() {
                break;
            }
            accepted += 1;
        }
        // Opportunistically start draining so a steady stream never relies on the next tick alone.
        let _ = self.pump()?;
        Ok(accepted)
    }
}
