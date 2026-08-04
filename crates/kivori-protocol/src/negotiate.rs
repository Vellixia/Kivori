//! Version + capability negotiation (data-model §3, FR-003). Pure functions over `kivori-model`
//! value types.

use kivori_model::Capabilities;

/// Negotiates the session's effective minor version and capability set from both peers.
///
/// The negotiated minor is the lower of the two minors; the negotiated capabilities are the
/// intersection (features both peers advertise).
#[must_use]
pub fn negotiate(
    desktop_minor: u16,
    device_minor: u16,
    desktop_caps: Capabilities,
    device_caps: Capabilities,
) -> (u16, Capabilities) {
    let minor = if desktop_minor < device_minor {
        desktop_minor
    } else {
        device_minor
    };
    (minor, desktop_caps.intersection(device_caps))
}
