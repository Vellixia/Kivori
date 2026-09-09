//! The production device runtime (T074): the real run loop, written against the three ports.
//!
//! This is **the** firmware behaviour. It is not a harness, not a self-test, and not simulation-specific:
//! the physical binary and the Wokwi `wokwi-runtime` mode call the same [`run`] with the same [`Runtime`],
//! differing only in which [`Clock`], [`Transport`], and [`DisplaySink`] instances they hand it and which
//! panel profile built the sink. There is deliberately no second loop implementation to drift.
//!
//! Each tick, in order:
//!
//! 1. finish boot once — `booting` → `offline`, the device-originated transition (FR-014/015);
//! 2. drain inbound bytes through the real decoder, sequence policy, and [`Dispatcher`], which answers
//!    `Hello`/`Ping`/`SetState` and drops malformed frames without side effects (SC-008);
//! 3. push any queued outbound bytes toward the hardware (the caller's transport may buffer);
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
use crate::ports::{Clock, DisplaySink, Transport};
use crate::proto::{DeviceIdentity, Dispatcher};
use crate::render::TileRenderer;
use crate::state::{DeviceEvent, DeviceState};
use kivori_assets::AssetBlob;
use kivori_model::{CompanionState, ElapsedMs, MascotAnimator};
use kivori_protocol::Message;

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
    /// A frame was composed on this tick, even if every tile matched the previous frame.
    pub frame_rendered: bool,
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

/// The production device runtime: lifecycle, protocol, rendering, and diagnostics.
pub struct Runtime<'a> {
    dispatcher: Dispatcher,
    device: DeviceState,
    renderer: TileRenderer<'a>,
    config: RuntimeConfig,
    booted: bool,
    next_frame_ms: ElapsedMs,
    next_health_ms: ElapsedMs,
    last_rendered: Option<CompanionState>,
    animator: MascotAnimator,
}

impl<'a> Runtime<'a> {
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
            animator: MascotAnimator::new(CompanionState::Booting, 0),
        }
    }

    /// Creates a runtime that composes a complete frame before starting display transfers.
    pub fn with_frame_buffer(
        identity: DeviceIdentity,
        config: RuntimeConfig,
        frame_buffer: &'a mut [kivori_model::Rgb565; crate::render::FRAME_PIXELS],
    ) -> Self {
        Self {
            renderer: TileRenderer::with_frame_buffer(frame_buffer),
            ..Self::new(identity, config)
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
    pub fn step<C, T, D>(
        &mut self,
        clock: &C,
        transport: &mut T,
        display: &mut D,
        blob: &AssetBlob,
    ) -> Tick
    where
        C: Clock,
        T: Transport,
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
            tick.link_dropped = true;
        }

        // Resolve changes immediately after protocol handling. Reporting remains semantic and does
        // not wait for the visual transition. Pixel hashes still detect which bands changed.
        let state = self.device.current();
        if self.animator.target() != state {
            self.animator.set_state(state, now);
        }
        // 3. Render on the frame cadence: only changed tiles reach the panel (FR-013).
        if now >= self.next_frame_ms {
            self.next_frame_ms =
                next_frame_deadline(self.next_frame_ms, now, self.config.frame_interval_ms);
            tick.frame_rendered = true;
            let state = self.device.current();
            if self.last_rendered != Some(state) {
                self.last_rendered = Some(state);
            }
            let mut counting = CountingSink {
                inner: display,
                flushes: 0,
                failed: false,
            };
            let pose = self.animator.pose_at(now);
            let outcome = self
                .renderer
                .render_animation(blob, state, &pose, &mut counting);
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

        // 4. At most one safe diagnostic per tick, category + code only (ADR-0005).
        if let Some(diagnostic) = self.dispatcher.take_diagnostic() {
            let message = Message::Diagnostic(build_diagnostic(diagnostic));
            if self.dispatcher.emit(transport, &message).is_ok() {
                tick.diagnostic = Some(diagnostic);
            }
        }

        // 5. Heartbeat health on a fixed cadence.
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

fn next_frame_deadline(
    previous_deadline_ms: ElapsedMs,
    now_ms: ElapsedMs,
    interval_ms: ElapsedMs,
) -> ElapsedMs {
    let interval_ms = interval_ms.max(1);
    let elapsed_intervals = now_ms.saturating_sub(previous_deadline_ms) / interval_ms;
    previous_deadline_ms.saturating_add(
        elapsed_intervals
            .saturating_add(1)
            .saturating_mul(interval_ms),
    )
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
pub fn run<C, T, D>(
    identity: DeviceIdentity,
    config: RuntimeConfig,
    clock: &C,
    transport: &mut T,
    display: &mut D,
    blob: &AssetBlob,
    observe: impl FnMut(&Tick, &mut T),
) -> !
where
    C: Clock,
    T: Transport,
    D: DisplaySink,
{
    let mut runtime = Runtime::new(identity, config);
    run_loop(&mut runtime, clock, transport, display, blob, observe)
}

/// Runs the same production loop with caller-owned frame staging memory.
#[allow(clippy::too_many_arguments)]
pub fn run_buffered<C, T, D>(
    identity: DeviceIdentity,
    config: RuntimeConfig,
    clock: &C,
    transport: &mut T,
    display: &mut D,
    blob: &AssetBlob,
    frame_buffer: &mut [kivori_model::Rgb565; crate::render::FRAME_PIXELS],
    observe: impl FnMut(&Tick, &mut T),
) -> !
where
    C: Clock,
    T: Transport,
    D: DisplaySink,
{
    let mut runtime = Runtime::with_frame_buffer(identity, config, frame_buffer);
    run_loop(&mut runtime, clock, transport, display, blob, observe)
}

fn run_loop<C, T, D>(
    runtime: &mut Runtime<'_>,
    clock: &C,
    transport: &mut T,
    display: &mut D,
    blob: &AssetBlob,
    mut observe: impl FnMut(&Tick, &mut T),
) -> !
where
    C: Clock,
    T: Transport,
    D: DisplaySink,
{
    loop {
        let tick = runtime.step(clock, transport, display, blob);
        // The observer receives the transport so a simulation mode can emit text markers over the same
        // link the protocol uses. The production binary passes a closure that does nothing.
        observe(&tick, transport);
    }
}
