//! Physical input: pure, host-testable decoding and gesture formation.
//!
//! Everything here sits ABOVE the `InputSource` port, so it is provable without hardware.

pub mod quadrature;
