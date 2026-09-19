//! The production device runtime (T074): the real run loop, written against the four ports.
//!
//! This is **the** firmware behaviour. It is not a harness, not a self-test, and not simulation-specific:
//! the physical binary and the Wokwi `wokwi-runtime` mode call the same [`run`] with the same [`Runtime`],
//! differing only in which [`Clock`], [`Transport`], [`InputSource`], and [`DisplaySink`] instances they
//! hand it and which panel profile built the sink. There is deliberately no second loop implementation to
//! drift.
//!
//! Each tick, in order:
//!
//! 1. finish boot once — `booting` → `offline`, the device-originated transition (FR-014/015);
//! 2. drain inbound bytes through the real decoder, sequence policy, and [`Dispatcher`], which answers
//!    `Hello`/`Ping`/`SetState` and drops malformed frames without side effects (SC-008); any session
//!    boundary — `Bye`, a transport failure, or a fresh `Hello` with no prior `Bye` — also resets the
//!    rotary decoder/gesture state so no gesture survives into a new (or recovered) session;
//! 3. sample physical input once, decode validated detents, and emit semantic `InputEvent`s — gated by
//!    the negotiated `PHYSICAL_INPUT_V1` capability and an accepted session inside the dispatcher;
//! 4. render the current state through the shared renderer, flushing only changed tiles (FR-013);
//! 5. emit at most one safe `Diagnostic` (allowlisted category + code — never payload bytes);
//! 6. emit `Health` on a fixed interval.
//!
//! The loop never returns and never panics on bad input: a transport failure is reported as a diagnostic
//! and the link drops to `offline`, from which a new `Hello` can bring the session back up.
//!
//! # What running this proves, and what it does not
//!
//! Executing it in a simulator exercises the real protocol, lifecycle, renderer, and tile-output paths on
//! the target ISA. It says nothing about the physical panel — its controller, offsets, orientation, colour
//! order, and backlight are supplied from outside this module and remain unconfirmed.

use crate::health::{build_diagnostic, build_health, DeviceDiagnostic};
use crate::input::gesture::{RotaryEvent, RotaryGesture};
use crate::input::quadrature::QuadratureDecoder;
use crate::ports::{Clock, DisplaySink, InputSource, Transport};
use crate::proto::{DeviceIdentity, Dispatcher};
use crate::render::TileRenderer;
use crate::state::{DeviceEvent, DeviceState};
use kivori_assets::AssetBlob;
use kivori_model::presentation::{PrimaryState, ValueDisplay};
use kivori_model::{CompanionState, ElapsedMs};
use kivori_protocol::{InputKind, Message, Nonce, Presentation};

/// Inactivity window, in milliseconds, after which an open rotary gesture ends
/// (user-story-contract section 5). Firmware-wide: both the production runtime and the host-sim
/// scenario helper (`sim::drive_rotary`) commit to this same boundary.
pub const GESTURE_END_MS: u32 = 250;

/// Loop timings. Both are integer milliseconds, so behaviour is deterministic (ADR-0003).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeConfig {
    /// Minimum gap between render passes. A pass still flushes nothing when no tile changed.
    pub frame_interval_ms: ElapsedMs,
    /// Gap between `Health` reports.
    pub health_interval_ms: ElapsedMs,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        // 30 fps is the canonical animation rate (ADR-0003); health once a second is enough for a
        // heartbeat without competing with frames for the FIFO.
        Self {
            frame_interval_ms: 33,
            health_interval_ms: 1000,
        }
    }
}

/// What one [`Runtime::step`] observed. Returned so tests and the Wokwi mode can assert on real work
/// instead of guessing from side effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Tick {
    /// The state the device holds after this tick.
    pub state: Option<CompanionState>,
    /// Tiles flushed to the display this tick.
    pub tiles_flushed: u32,
    /// A safe diagnostic was transmitted.
    pub diagnostic: Option<DeviceDiagnostic>,
    /// A `Health` report was transmitted.
    pub health_sent: bool,
    /// The transport reported a failure and the link was dropped.
    pub link_dropped: bool,
    /// Frames the decoder has rejected since boot (cumulative).
    pub rejected_frames: u32,
    /// `HelloAck` replies transmitted since boot (cumulative).
    pub hello_acks: u32,
    /// `Pong` replies transmitted since boot (cumulative).
    pub pongs: u32,
    /// `StateReport` messages transmitted since boot (cumulative).
    pub state_reports: u32,
}

/// A [`DisplaySink`] decorator counting flushes, so the runtime can report tile activity without the sink
/// having to.
struct CountingSink<'s, S> {
    inner: &'s mut S,
    flushes: u32,
    failed: bool,
}

impl<S: DisplaySink> DisplaySink for CountingSink<'_, S> {
    type Error = S::Error;

    fn blit_tile(
        &mut self,
        rect: kivori_model::Rect,
        pixels: &[kivori_model::Rgb565],
    ) -> Result<(), Self::Error> {
        match self.inner.blit_tile(rect, pixels) {
            Ok(()) => {
                self.flushes += 1;
                Ok(())
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
}

/// Device-side presentation acceptance and local transient expiry.
///
/// Firmware expires the overlay itself so it cannot stick if the host disappears mid-transient,
/// restoring the underlying `primary` (contract invariant 50).
#[derive(Debug)]
pub struct PresentationState {
    session: Option<Nonce>,
    last_revision: u32,
    primary: PrimaryState,
    value: Option<ValueDisplay>,
    /// The device-ms timestamp `value` was applied at. Meaningless when `transient_ms == 0`
    /// (persistent) or `value` is `None`.
    applied_at_ms: u32,
    /// `0` = persistent; otherwise `value` expires this many ms after `applied_at_ms`, compared
    /// via `wrapping_sub` (the same idiom `RotaryGesture`'s inactivity window uses) rather than a
    /// precomputed absolute deadline — a precomputed `applied_at_ms + transient_ms` can itself
    /// wrap past `u32::MAX` (~49.7 days of device uptime) while `now_ms` has not yet wrapped,
    /// which would make a plain `now_ms >= deadline` comparison see a small deadline and a huge
    /// `now_ms` and report the overlay expired instantly.
    transient_ms: u16,
}

impl PresentationState {
    /// A fresh presentation state, before any session is accepted.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            session: None,
            last_revision: 0,
            primary: PrimaryState::Idle,
            value: None,
            applied_at_ms: 0,
            transient_ms: 0,
        }
    }

    /// A new compatible host session: rebind identity and reset revision scoping.
    pub fn begin_session(&mut self, session: Nonce) {
        self.session = Some(session);
        self.last_revision = 0;
        self.value = None;
        self.transient_ms = 0;
    }

    /// Ends the current session: no session, no lingering overlay.
    pub fn end_session(&mut self) {
        self.session = None;
        self.last_revision = 0;
        self.value = None;
        self.transient_ms = 0;
    }

    /// Returns true when the presentation was accepted and applied.
    pub fn apply(&mut self, p: &Presentation, now_ms: u32) -> bool {
        if self.session != Some(p.session) {
            return false;
        }
        if p.revision <= self.last_revision {
            return false;
        }
        self.last_revision = p.revision;
        self.primary = p.primary;
        self.value = p.value;
        self.applied_at_ms = now_ms;
        self.transient_ms = if p.value.is_some() { p.transient_ms } else { 0 };
        true
    }

    /// The overlay still in force at `now_ms`, if any.
    #[must_use]
    pub fn value_at(&self, now_ms: u32) -> Option<ValueDisplay> {
        if self.transient_ms == 0 {
            return self.value;
        }
        // Wrap-aware elapsed time (see the `transient_ms` field doc): correct across a device
        // uptime rollover, unlike comparing `now_ms` against a precomputed absolute deadline.
        if now_ms.wrapping_sub(self.applied_at_ms) >= u32::from(self.transient_ms) {
            None
        } else {
            self.value
        }
    }

    /// The underlying truth beneath any transient overlay.
    #[must_use]
    pub const fn primary(&self) -> PrimaryState {
        self.primary
    }
}

impl Default for PresentationState {
    fn default() -> Self {
        Self::new()
    }
}

/// The production device runtime: lifecycle, protocol, rendering, and diagnostics.
pub struct Runtime {
    dispatcher: Dispatcher,
    device: DeviceState,
    renderer: TileRenderer,
    config: RuntimeConfig,
    booted: bool,
    next_frame_ms: ElapsedMs,
    next_health_ms: ElapsedMs,
    last_rendered: Option<CompanionState>,
    /// Turns raw quadrature levels into validated logical detents (never raw electrical edges).
    decoder: QuadratureDecoder,
    /// Groups validated detents into gestures (identity + the 250 ms inactivity boundary).
    gesture: RotaryGesture,
    /// Session-scoped acceptance and local expiry of the device-rendered `Presentation` overlay.
    presentation: PresentationState,
}

impl Runtime {
    /// Creates the runtime for a device advertising `identity`.
    #[must_use]
    pub fn new(identity: DeviceIdentity, config: RuntimeConfig) -> Self {
        Self {
            dispatcher: Dispatcher::new(identity),
            device: DeviceState::new(),
            renderer: TileRenderer::new(),
            config,
            booted: false,
            next_frame_ms: 0,
            next_health_ms: 0,
            last_rendered: None,
            decoder: QuadratureDecoder::new(),
            gesture: RotaryGesture::new(GESTURE_END_MS),
            presentation: PresentationState::new(),
        }
    }

    /// The companion state the device currently holds.
    #[must_use]
    pub const fn state(&self) -> CompanionState {
        self.device.current()
    }

    /// Frames the decoder has rejected this session.
    #[must_use]
    pub const fn rejected_frames(&self) -> u32 {
        self.dispatcher.rejected_frames()
    }

    /// Runs one tick of the production loop.
    ///
    /// Never fails: a transport error becomes a `LinkLost` diagnostic and an `offline` transition, because a
    /// device that gives up on a bad cable is worse than one that waits for the host to come back.
    pub fn step<C, T, I, D>(
        &mut self,
        clock: &C,
        transport: &mut T,
        input: &mut I,
        display: &mut D,
        blob: &AssetBlob,
    ) -> Tick
    where
        C: Clock,
        T: Transport,
        I: InputSource,
        D: DisplaySink,
    {
        let mut tick = Tick::default();
        let now = clock.now_ms();

        // 1. The device owns `booting` at power-on and falls back to `offline` with no host present.
        if !self.booted {
            self.booted = true;
            tick.state = self.device.apply(DeviceEvent::BootComplete);
        }

        // 2. Inbound: real framing, CRC, sequence policy, and dispatch. Malformed frames are dropped
        //    inside the dispatcher and surface below as a diagnostic.
        if self
            .dispatcher
            .poll(transport, &mut self.device, now)
            .is_err()
        {
            let _ = self.device.apply(DeviceEvent::LinkDown);
            self.dispatcher.note_diagnostic(DeviceDiagnostic::LinkLost);
            // A transport failure is a session boundary too, even with no `Bye`: clear the
            // accepted session/negotiated capability so a stale gesture cannot keep emitting
            // once the link recovers.
            self.dispatcher.link_lost();
            tick.link_dropped = true;
        }
        // A `Bye`, a transport failure, or a new `Hello` this poll closed/opened a session: a
        // gesture (or partial motion) from the old session must never complete in a new one
        // (no-stale-replay invariant).
        if self.dispatcher.take_session_ended() {
            self.decoder.reset();
            self.gesture.reset();
            // A session boundary is also a presentation-scoping boundary: `revision` is strictly
            // increasing WITHIN a session and resets with it, so a restarted desktop starting
            // again at revision 1 is never rejected as stale traffic from the old, higher-revision
            // session. `accepted_session()` tells us whether this boundary opened a new session
            // (`Some`) or closed one (`None`).
            match self.dispatcher.accepted_session() {
                Some(session) => self.presentation.begin_session(session),
                None => self.presentation.end_session(),
            }
        }

        // Apply any `Presentation` the dispatcher accepted this poll. Capability negotiation and
        // session/revision freshness are already enforced by the dispatcher and `PresentationState`
        // themselves, so this is unconditional.
        if let Some(presentation) = self.dispatcher.take_presentation() {
            self.presentation.apply(&presentation, now);
        }

        // 3. Physical input: sample once per tick, turn validated detents into gestures, and emit
        //    semantic `InputEvent`s. `send_input_event` itself gates on capability/session, so this
        //    stays silent until the desktop has negotiated `PHYSICAL_INPUT_V1`.
        let levels = input.sample();
        if let Some(direction) = self.decoder.update(levels.a, levels.b) {
            let (started, detent) = self.gesture.on_detent(direction, now);
            if let Some(RotaryEvent::GestureStarted { gesture_id }) = started {
                self.dispatcher.send_input_event(
                    transport,
                    gesture_id,
                    InputKind::GestureStarted,
                    now,
                );
            }
            if let RotaryEvent::Detent {
                gesture_id,
                direction,
            } = detent
            {
                self.dispatcher.send_input_event(
                    transport,
                    gesture_id,
                    InputKind::Detent(direction),
                    now,
                );
            }
        }
        if let Some(RotaryEvent::GestureEnded { gesture_id }) = self.gesture.poll(now) {
            self.dispatcher
                .send_input_event(transport, gesture_id, InputKind::GestureEnded, now);
        }

        // 4. Render on the frame cadence: only changed tiles reach the panel (FR-013). A state change
        //    invalidates the cache, because the previous frame's tiles belong to a different scene.
        if now >= self.next_frame_ms {
            self.next_frame_ms = now.saturating_add(self.config.frame_interval_ms);
            let state = self.device.current();
            if self.last_rendered != Some(state) {
                self.renderer.invalidate();
                self.last_rendered = Some(state);
            }
            let mut counting = CountingSink {
                inner: display,
                flushes: 0,
                failed: false,
            };
            // The transient overlay, if still in force; expired locally back to `None` so the
            // panel shows plain `state` once it lapses, with no host timer or round trip needed.
            let overlay = self.presentation.value_at(now);
            let outcome =
                self.renderer
                    .render_with_overlay(blob, state, now, overlay, &mut counting);
            tick.tiles_flushed = counting.flushes;
            let failed = counting.failed;
            if outcome.is_err() {
                // A sink failure is reportable; a missing scene is a build-time bug that must not spin.
                if failed {
                    self.dispatcher
                        .note_diagnostic(DeviceDiagnostic::DisplayFault);
                }
                self.renderer.invalidate();
            }
        }

        // 5. At most one safe diagnostic per tick, category + code only (ADR-0005).
        if let Some(diagnostic) = self.dispatcher.take_diagnostic() {
            let message = Message::Diagnostic(build_diagnostic(diagnostic));
            if self.dispatcher.emit(transport, &message).is_ok() {
                tick.diagnostic = Some(diagnostic);
            }
        }

        // 6. Heartbeat health on a fixed cadence.
        if now >= self.next_health_ms {
            self.next_health_ms = now.saturating_add(self.config.health_interval_ms);
            let message = Message::Health(build_health(free_bytes()));
            if self.dispatcher.emit(transport, &message).is_ok() {
                tick.health_sent = true;
            }
        }

        if tick.state.is_none() {
            tick.state = Some(self.device.current());
        }
        tick.rejected_frames = self.dispatcher.rejected_frames();
        tick.hello_acks = self.dispatcher.hello_acks();
        tick.pongs = self.dispatcher.pongs();
        tick.state_reports = self.dispatcher.state_reports();
        tick
    }
}

/// Free SRAM in bytes.
///
/// Measuring genuine heap/stack headroom needs linker symbols and a stack-watermark scheme that only means
/// anything on real silicon, so this reports `0` — "not measured" — rather than a fabricated number that
/// would read as a real measurement on a dashboard. Producing a true figure is part of the on-device
/// validation work (T116/T119), not something simulation can establish.
const fn free_bytes() -> u32 {
    0
}

/// Runs the production loop forever.
///
/// The physical binary and the Wokwi runtime mode both call this; only the injected ports differ.
// One argument per port/config/observer — bundling them would obscure which port is which at every
// call site for no real benefit here.
#[allow(clippy::too_many_arguments)]
pub fn run<C, T, I, D>(
    identity: DeviceIdentity,
    config: RuntimeConfig,
    clock: &C,
    transport: &mut T,
    input: &mut I,
    display: &mut D,
    blob: &AssetBlob,
    mut observe: impl FnMut(&Tick, &mut T),
) -> !
where
    C: Clock,
    T: Transport,
    I: InputSource,
    D: DisplaySink,
{
    let mut runtime = Runtime::new(identity, config);
    loop {
        let tick = runtime.step(clock, transport, input, display, blob);
        // The observer receives the transport so a simulation mode can emit text markers over the same
        // link the protocol uses. The production binary passes a closure that does nothing.
        observe(&tick, transport);
    }
}
