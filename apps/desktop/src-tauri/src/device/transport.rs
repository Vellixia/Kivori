//! The serial transport abstraction (research R-5).
//!
//! The connection manager and heartbeat drive I/O through this trait so the logic is testable against
//! an in-memory double. The production adapter (tokio-serial primary; blocking `serialport` +
//! `spawn_blocking` fallback) implements it in the runtime wiring phase.

/// A bounded, non-blocking byte link to a device (the desktop counterpart to the firmware transport).
pub trait SerialLink {
    /// Link-specific error type. Any error is surfaced to the manager as a connection `IoError`.
    type Error: core::fmt::Debug;

    /// Reads any immediately-available bytes into `buf`, returning the count (`0` if none). Never
    /// blocks.
    ///
    /// # Errors
    /// Returns [`Self::Error`] on an unrecoverable read failure.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    /// Writes as many bytes of `buf` as fit without blocking, returning the count accepted.
    ///
    /// # Errors
    /// Returns [`Self::Error`] on an unrecoverable write failure.
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;
}
