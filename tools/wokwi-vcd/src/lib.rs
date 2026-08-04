//! Deterministic VCD checker for the Wokwi generic SPI display probe (T131, step 8).
//!
//! Parses a Value Change Dump and asserts **observable digital** properties of the SPI bus:
//!
//! * clock activity exists on SCK
//! * CS transitions exist (transactions are framed)
//! * D/C transitions exist (command and data phases are distinguishable)
//! * a reset pulse exists, and it precedes the first bus traffic
//! * data is transmitted *after* a command phase (D/C low with clocking, then D/C high with clocking)
//! * a minimum transfer volume occurs (SCK edges)
//! * the stage ordering is monotonic: reset → first command phase → first data phase → a later burst
//!
//! # What a VCD cannot prove
//!
//! Nothing analog: no signal integrity, no rise/fall times, no voltage levels, no timing margin, no EMI.
//! A VCD is a digital event list, and this checker never claims otherwise. It also proves nothing about
//! any real display controller — the probe drives Wokwi's generic display part.
//!
//! # Signal naming is discovered, not assumed
//!
//! Wokwi's VCD signal names for a `board-esp32-c3-devkitm-1` net are not documented, so role resolution
//! tries a list of candidate substrings per role and, when a role cannot be resolved, **fails loudly and
//! prints every signal name the file actually declares**. That output is the evidence needed to adapt the
//! candidates — the checker never degrades into "the file exists".

use std::collections::BTreeMap;
use std::fmt;

/// A bus line the checker needs to identify.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// SPI clock.
    Sck,
    /// SPI data out (MCU → panel).
    Mosi,
    /// Chip select.
    Cs,
    /// Data/command select.
    Dc,
    /// Panel reset.
    Reset,
}

impl Role {
    /// Every role the checker resolves.
    pub const ALL: [Role; 5] = [Role::Sck, Role::Mosi, Role::Cs, Role::Dc, Role::Reset];

    /// Human-readable role name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Role::Sck => "sck",
            Role::Mosi => "mosi",
            Role::Cs => "cs",
            Role::Dc => "dc",
            Role::Reset => "reset",
        }
    }

    /// Candidate signal-name substrings, most specific first.
    ///
    /// Both endpoints of each net are listed because a VCD may name a signal after either the board pin
    /// or the part pin. Matching is case-insensitive.
    #[must_use]
    pub fn candidates(self) -> &'static [&'static str] {
        match self {
            Role::Sck => &["lcd:sck", "sck", "sclk", "esp:4", "gpio4"],
            Role::Mosi => &["lcd:mosi", "mosi", "sdi", "esp:5", "gpio5"],
            Role::Cs => &["lcd:cs", "cs", "esp:6", "gpio6"],
            Role::Dc => &["lcd:d/c", "d/c", "dc", "esp:7", "gpio7"],
            Role::Reset => &["lcd:rst", "rst", "reset", "esp:10", "gpio10"],
        }
    }
}

/// A single-bit transition: simulation time and new level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    /// Simulation timestamp in timescale units.
    pub time: u64,
    /// New level (`true` = high). Unknown/high-impedance values are recorded as `false`.
    pub level: bool,
}

/// A parsed VCD: declared signals plus their transitions.
#[derive(Debug, Default)]
pub struct Vcd {
    /// Declared signal names, in declaration order.
    pub signals: Vec<String>,
    /// Transitions per declared signal name.
    pub transitions: BTreeMap<String, Vec<Transition>>,
    /// The `$timescale` value, verbatim, if present.
    pub timescale: Option<String>,
}

/// Why parsing or checking failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VcdError {
    /// The file declared no signals at all (not a usable VCD).
    NoSignals,
    /// `$enddefinitions` never appeared, so the header is truncated.
    TruncatedHeader,
    /// A role could not be matched to any declared signal.
    UnresolvedRole {
        /// The role that could not be resolved.
        role: Role,
        /// Every signal name the file declared, for adapting the candidate list from evidence.
        available: Vec<String>,
    },
}

impl fmt::Display for VcdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VcdError::NoSignals => write!(f, "no $var declarations found — not a usable VCD"),
            VcdError::TruncatedHeader => write!(f, "no $enddefinitions — VCD header is truncated"),
            VcdError::UnresolvedRole { role, available } => write!(
                f,
                "could not resolve the '{}' signal. Tried: {:?}. Signals present in the VCD: {:?}",
                role.name(),
                role.candidates(),
                available
            ),
        }
    }
}

/// Parses a VCD.
///
/// Handles the subset every VCD writer emits: `$var` declarations inside optional `$scope`s, scalar value
/// changes (`0!`, `1!`, `x!`, `z!`), vector changes (`b1010 !`), and `#<time>` timestamps. Unknown
/// (`x`/`z`) levels are recorded as low so a stuck-unknown line cannot be mistaken for activity.
///
/// # Errors
/// [`VcdError::NoSignals`] or [`VcdError::TruncatedHeader`] if the file is not a usable VCD.
pub fn parse(text: &str) -> Result<Vcd, VcdError> {
    let mut vcd = Vcd::default();
    let mut ids: BTreeMap<String, String> = BTreeMap::new();
    let mut time: u64 = 0;
    let mut in_definitions = true;
    let mut saw_enddefinitions = false;

    let mut tokens = text.split_whitespace().peekable();
    while let Some(token) = tokens.next() {
        if in_definitions {
            match token {
                "$var" => {
                    // $var <type> <width> <id> <name...> $end
                    let _kind = tokens.next();
                    let _width = tokens.next();
                    let id = tokens.next().unwrap_or_default().to_string();
                    let mut name_parts: Vec<&str> = Vec::new();
                    for part in tokens.by_ref() {
                        if part == "$end" {
                            break;
                        }
                        name_parts.push(part);
                    }
                    let name = name_parts.join(" ");
                    if !id.is_empty() && !name.is_empty() {
                        ids.insert(id, name.clone());
                        vcd.signals.push(name.clone());
                        vcd.transitions.entry(name).or_default();
                    }
                    continue;
                }
                "$timescale" => {
                    let mut parts: Vec<&str> = Vec::new();
                    for part in tokens.by_ref() {
                        if part == "$end" {
                            break;
                        }
                        parts.push(part);
                    }
                    vcd.timescale = Some(parts.join(" "));
                    continue;
                }
                "$enddefinitions" => {
                    // consume through $end
                    for part in tokens.by_ref() {
                        if part == "$end" {
                            break;
                        }
                    }
                    in_definitions = false;
                    saw_enddefinitions = true;
                    continue;
                }
                _ => continue,
            }
        }

        if let Some(rest) = token.strip_prefix('#') {
            if let Ok(t) = rest.parse::<u64>() {
                time = t;
            }
            continue;
        }
        // Scalar change: one value character immediately followed by the identifier.
        let mut chars = token.chars();
        let Some(first) = chars.next() else { continue };
        match first {
            '0' | '1' | 'x' | 'X' | 'z' | 'Z' => {
                let id: String = chars.collect();
                if id.is_empty() {
                    continue;
                }
                push(&mut vcd, &ids, &id, time, first == '1');
            }
            'b' | 'B' => {
                // Vector change: `b<bits> <id>` — treat non-zero as high.
                let bits: String = chars.collect();
                if let Some(id) = tokens.next() {
                    let high = bits.chars().any(|c| c == '1');
                    push(&mut vcd, &ids, id, time, high);
                }
            }
            _ => {}
        }
    }

    if vcd.signals.is_empty() {
        return Err(VcdError::NoSignals);
    }
    if !saw_enddefinitions {
        return Err(VcdError::TruncatedHeader);
    }
    Ok(vcd)
}

fn push(vcd: &mut Vcd, ids: &BTreeMap<String, String>, id: &str, time: u64, level: bool) {
    if let Some(name) = ids.get(id) {
        vcd.transitions
            .entry(name.clone())
            .or_default()
            .push(Transition { time, level });
    }
}

impl Vcd {
    /// Resolves `role` to a declared signal name.
    ///
    /// # Errors
    /// [`VcdError::UnresolvedRole`] listing every declared signal, so the candidate list can be adapted
    /// from an actual capture rather than from guesswork.
    pub fn resolve(&self, role: Role) -> Result<&str, VcdError> {
        for candidate in role.candidates() {
            if let Some(name) = self
                .signals
                .iter()
                .find(|name| name.to_ascii_lowercase().contains(candidate))
            {
                return Ok(name);
            }
        }
        Err(VcdError::UnresolvedRole {
            role,
            available: self.signals.clone(),
        })
    }

    /// Level changes recorded for `role`.
    ///
    /// # Errors
    /// [`VcdError::UnresolvedRole`] if the role cannot be resolved.
    pub fn edges(&self, role: Role) -> Result<&[Transition], VcdError> {
        let name = self.resolve(role)?;
        Ok(self
            .transitions
            .get(name)
            .map_or(&[][..], |edges| edges.as_slice()))
    }
}

/// One checked property and its outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Short stable name (used in CI output).
    pub name: &'static str,
    /// Whether the property held.
    pub ok: bool,
    /// Measured evidence for the property.
    pub detail: String,
}

/// Thresholds the checker applies. Kept explicit so CI cannot silently accept a thinner capture.
#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    /// Minimum SCK rising edges (one per clocked bit).
    pub min_clock_edges: usize,
    /// Minimum CS level changes.
    pub min_cs_transitions: usize,
    /// Minimum D/C level changes.
    pub min_dc_transitions: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        // One 240x40 RGB565 tile is 19,200 pixel bytes = 153,600 clocked bits. Requiring a hundred
        // thousand rising edges therefore cannot be satisfied by init traffic alone, but stays well below
        // a full six-tile frame so a truncated capture window is not mistaken for a bus failure.
        Self {
            min_clock_edges: 100_000,
            min_cs_transitions: 8,
            min_dc_transitions: 6,
        }
    }
}

/// Counts rising edges (low → high), ignoring repeated same-level entries.
#[must_use]
pub fn rising_edges(edges: &[Transition]) -> usize {
    let mut count = 0;
    let mut last = false;
    for edge in edges {
        if edge.level && !last {
            count += 1;
        }
        last = edge.level;
    }
    count
}

/// Counts level changes.
#[must_use]
pub fn transitions(edges: &[Transition]) -> usize {
    let mut count = 0;
    let mut last: Option<bool> = None;
    for edge in edges {
        if last != Some(edge.level) {
            if last.is_some() {
                count += 1;
            }
            last = Some(edge.level);
        }
    }
    count
}

/// The first time `role` goes low then high again (a reset pulse), if any.
#[must_use]
pub fn pulse_low_then_high(edges: &[Transition]) -> Option<(u64, u64)> {
    let low = edges.iter().find(|e| !e.level)?;
    let high = edges.iter().find(|e| e.level && e.time >= low.time)?;
    Some((low.time, high.time))
}

/// Returns the first clock edge at or after `after` that occurs while `gate` holds `level`.
///
/// The gate level comes from the FULL gate history (a level set before the window still holds inside it).
#[must_use]
pub fn first_clock_while(
    clock: &[Transition],
    gate: &[Transition],
    level: bool,
    after: u64,
) -> Option<u64> {
    let gate_level_at =
        |t: u64| -> Option<bool> { gate.iter().rev().find(|g| g.time <= t).map(|g| g.level) };
    clock
        .iter()
        .filter(|c| c.level && c.time >= after)
        .find(|c| gate_level_at(c.time) == Some(level))
        .map(|c| c.time)
}

/// Transitions at or after `after`.
///
/// Needed because a real Wokwi capture records every line as `1` at time 0 — the pins float high until the
/// firmware configures them, ~164 ms into boot. Counting that pre-bring-up sample as bus activity makes
/// "data before any command" look true on a perfectly good capture, so every activity measurement is taken
/// inside the post-reset window.
#[must_use]
pub fn since(edges: &[Transition], after: u64) -> Vec<Transition> {
    edges.iter().copied().filter(|e| e.time >= after).collect()
}

/// Runs every check against `vcd`.
///
/// # Errors
/// [`VcdError::UnresolvedRole`] if any required signal cannot be identified — an unresolved capture is a
/// failure, never a pass.
pub fn check(vcd: &Vcd, thresholds: Thresholds) -> Result<Vec<Finding>, VcdError> {
    // Resolve every role first, so an unnameable capture fails before any property is "measured".
    for role in Role::ALL {
        vcd.resolve(role)?;
    }
    let sck_all = vcd.edges(Role::Sck)?;
    let mosi_all = vcd.edges(Role::Mosi)?;
    let cs_all = vcd.edges(Role::Cs)?;
    let dc_all = vcd.edges(Role::Dc)?;
    let reset = vcd.edges(Role::Reset)?;

    // Everything is measured after the panel reset is released: that is the bring-up boundary, and it
    // excludes the floating-high sample every line carries at time 0.
    let reset_pulse = pulse_low_then_high(reset);
    let window = reset_pulse.map_or(0, |(_, released)| released);
    let sck = since(sck_all, window);
    let mosi = since(mosi_all, window);
    let cs = since(cs_all, window);
    let dc = since(dc_all, window);

    let clock_edges = rising_edges(&sck);
    let cs_changes = transitions(&cs);
    let dc_changes = transitions(&dc);
    let mosi_changes = transitions(&mosi);
    let first_command = first_clock_while(sck_all, dc_all, false, window);
    let first_data = first_clock_while(sck_all, dc_all, true, window);
    let last_clock = sck.iter().rfind(|e| e.level).map(|e| e.time);

    let mut findings = vec![
        Finding {
            name: "clock-activity",
            ok: clock_edges > 0,
            detail: format!("{clock_edges} SCK rising edges"),
        },
        Finding {
            name: "cs-transitions",
            ok: cs_changes >= thresholds.min_cs_transitions,
            detail: format!(
                "{cs_changes} CS changes (need >= {})",
                thresholds.min_cs_transitions
            ),
        },
        Finding {
            name: "dc-transitions",
            ok: dc_changes >= thresholds.min_dc_transitions,
            detail: format!(
                "{dc_changes} D/C changes (need >= {})",
                thresholds.min_dc_transitions
            ),
        },
        Finding {
            name: "mosi-activity",
            ok: mosi_changes > 0,
            detail: format!("{mosi_changes} MOSI changes"),
        },
        Finding {
            name: "reset-sequence",
            ok: reset_pulse.is_some(),
            detail: match reset_pulse {
                Some((low, high)) => format!("reset low at {low}, released at {high}"),
                None => "no low→high reset pulse".to_string(),
            },
        },
        Finding {
            name: "transfer-volume",
            ok: clock_edges >= thresholds.min_clock_edges,
            detail: format!(
                "{clock_edges} clocked bits (need >= {})",
                thresholds.min_clock_edges
            ),
        },
    ];

    // Data must follow a command phase: both must exist, and the first command-phase clock must not come
    // after the first data-phase clock.
    findings.push(Finding {
        name: "data-follows-command",
        ok: match (first_command, first_data) {
            (Some(cmd), Some(data)) => cmd <= data,
            _ => false,
        },
        detail: format!("first command clock {first_command:?}, first data clock {first_data:?}"),
    });

    // Stage ordering: reset released before the first clock, and traffic continues past the first burst.
    findings.push(Finding {
        name: "stage-ordering",
        ok: match (reset_pulse, first_command, last_clock) {
            (Some((_, released)), Some(cmd), Some(last)) => released <= cmd && last > cmd,
            _ => false,
        },
        detail: format!(
            "reset={reset_pulse:?} first-command={first_command:?} last-clock={last_clock:?}"
        ),
    });

    Ok(findings)
}
