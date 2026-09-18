//! Preview ownership and reconciliation for the continuous volume value.
//!
//! While a gesture is open the local target owns the display (Preview). External
//! changes are recorded but never rendered over an active gesture. On gesture end
//! the value reconciles to what the backend reports (Confirmed) — desktop truth
//! wins (user-story-contract invariants 1 and 30, US3, US4).

use super::volume::apply_step;
use super::{execute_volume, Outcome};
use crate::input::LogicalInput;
use crate::platform::VolumeBackend;
use kivori_model::presentation::ValueConfidence;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueUpdate {
    pub percent: u8,
    pub confidence: ValueConfidence,
    pub at_boundary: bool,
}

#[derive(Debug, Default)]
pub struct GestureValue {
    /// `Some(gesture_id)` while a gesture owns the preview.
    active: Option<u16>,
    /// Gesture-local target, seeded from the backend when the gesture opens.
    target: u8,
    /// Set when an endpoint rebind invalidated the in-flight gesture.
    abandoned: bool,
}

impl GestureValue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_input(
        &mut self,
        input: LogicalInput,
        backend: &dyn VolumeBackend,
    ) -> Option<ValueUpdate> {
        match input {
            LogicalInput::GestureStarted { gesture_id } => {
                self.active = Some(gesture_id);
                self.abandoned = false;
                self.target = backend.read().unwrap_or(self.target);
                None
            }
            LogicalInput::Detent {
                gesture_id,
                direction,
            } => {
                if self.active != Some(gesture_id) || self.abandoned {
                    return None;
                }
                let (next, at_boundary) = apply_step(self.target, direction);
                self.target = next;

                let confidence = match execute_volume(backend, next) {
                    // A confirmed write still displays as Preview while the gesture
                    // owns the surface; it is promoted at gesture end.
                    Outcome::StateConfirmed { .. } => ValueConfidence::Preview,
                    Outcome::TriggeredUnverified => ValueConfidence::Unverified,
                    _ => ValueConfidence::Unverified,
                };

                Some(ValueUpdate {
                    percent: next,
                    confidence,
                    at_boundary,
                })
            }
            LogicalInput::GestureEnded { gesture_id } => {
                if self.active != Some(gesture_id) {
                    return None;
                }
                self.active = None;
                if self.abandoned {
                    self.abandoned = false;
                    return None;
                }
                let observed = backend.read().ok()?;
                self.target = observed;
                Some(ValueUpdate {
                    percent: observed,
                    confidence: ValueConfidence::Confirmed,
                    at_boundary: false,
                })
            }
        }
    }

    /// A change Kivori did not originate. Ignored while a gesture owns the preview.
    pub fn on_external_change(&mut self, percent: u8) -> Option<ValueUpdate> {
        if self.active.is_some() {
            return None;
        }
        self.target = percent;
        Some(ValueUpdate {
            percent,
            confidence: ValueConfidence::Confirmed,
            at_boundary: false,
        })
    }

    /// The default render endpoint changed. The value being controlled is now a
    /// different thing, so an in-flight gesture is abandoned rather than retargeted.
    pub fn on_endpoint_rebind(&mut self, percent: u8) -> Option<ValueUpdate> {
        if self.active.is_some() {
            self.abandoned = true;
        }
        self.target = percent;
        Some(ValueUpdate {
            percent,
            confidence: ValueConfidence::Confirmed,
            at_boundary: false,
        })
    }
}
