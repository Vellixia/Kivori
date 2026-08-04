//! Candidate serial-port discovery (FR-001, FR-036).
//!
//! Kivori devices are found by USB VID/PID, never a fixed COM port. Port enumeration itself lives in
//! the serial adapter; this module holds the pure, testable filter over enumerated ports.

/// A `(vendor, product)` USB id pair.
pub type UsbId = (u16, u16);

/// Espressif's USB vendor id (the ESP32-C3 USB Serial/JTAG).
pub const KIVORI_VID: u16 = 0x303A;
/// Kivori's USB product id.
pub const KIVORI_PID: u16 = 0x1001;

/// The default VID/PID allowlist (a single Kivori id; widen it to onboard new hardware).
pub const DEFAULT_ALLOWLIST: &[UsbId] = &[(KIVORI_VID, KIVORI_PID)];

/// An enumerated serial port and its USB identity (absent for non-USB ports).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortCandidate {
    /// OS port name (e.g. `COM7`, `/dev/ttyACM0`). Opaque — never the selection key.
    pub port_name: String,
    /// USB vendor id, if the port is a USB device.
    pub vid: Option<u16>,
    /// USB product id, if the port is a USB device.
    pub pid: Option<u16>,
}

impl PortCandidate {
    /// Creates a candidate.
    #[must_use]
    pub fn new(port_name: impl Into<String>, vid: Option<u16>, pid: Option<u16>) -> Self {
        Self {
            port_name: port_name.into(),
            vid,
            pid,
        }
    }
}

/// Whether `port` matches the allowlist (both VID and PID must be present and listed).
#[must_use]
pub fn is_candidate(port: &PortCandidate, allowlist: &[UsbId]) -> bool {
    matches!((port.vid, port.pid), (Some(v), Some(p)) if allowlist.contains(&(v, p)))
}

/// Returns the subset of `ports` matching `allowlist`, preserving order.
#[must_use]
pub fn filter_candidates<'a>(
    ports: &'a [PortCandidate],
    allowlist: &[UsbId],
) -> Vec<&'a PortCandidate> {
    ports
        .iter()
        .filter(|p| is_candidate(p, allowlist))
        .collect()
}
