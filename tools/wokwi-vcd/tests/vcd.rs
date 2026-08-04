//! Tests for the VCD checker.
//!
//! Every property has a negative control: a mutated capture that must make exactly that property fail.
//! Without those, a checker can pass on anything.

use kivori_wokwi_vcd::{
    check, first_clock_while, parse, rising_edges, since, transitions, Role, Thresholds,
    Transition, VcdError,
};

/// VCD identifier characters for the five signals.
const IDS: [(&str, &str); 5] = [
    ("!", "lcd:SCK"),
    ("\"", "lcd:MOSI"),
    ("#", "lcd:CS"),
    ("$", "lcd:D/C"),
    ("%", "lcd:RST"),
];

fn header() -> String {
    let mut out = String::from("$timescale 1us $end\n$scope module wokwi $end\n");
    for (id, name) in IDS {
        out.push_str(&format!("$var wire 1 {id} {name} $end\n"));
    }
    out.push_str("$upscope $end\n$enddefinitions $end\n");
    out
}

/// Shape of a synthetic capture.
struct Shape {
    /// Emit the reset low→high pulse.
    reset: bool,
    /// Emit a D/C-low (command) burst before the data burst.
    command_phase: bool,
    /// Clocked bits per burst.
    bits: usize,
    /// Number of CS-framed transactions.
    transactions: usize,
    /// Emit the all-lines-high sample at time 0 that a real Wokwi capture records before the firmware
    /// configures its pins.
    floating_prefix: bool,
    /// Simulation time the real traffic starts at.
    offset: u64,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            reset: true,
            command_phase: true,
            bits: 40,
            transactions: 6,
            floating_prefix: false,
            offset: 0,
        }
    }
}

/// Builds a synthetic capture: optional reset pulse, then CS-framed transactions that clock a command
/// phase (D/C low) followed by a data phase (D/C high).
fn capture(shape: &Shape) -> String {
    let mut out = header();
    if shape.floating_prefix {
        // Every line floats high until the firmware drives it.
        out.push_str("#0\n1!\n1\"\n1#\n1$\n1%\n");
    }
    let mut t: u64 = shape.offset;
    out.push_str(&format!("#{t}\n0!\n0\"\n1#\n0$\n1%\n"));
    if shape.reset {
        t += 10;
        out.push_str(&format!("#{t}\n0%\n"));
        t += 10;
        out.push_str(&format!("#{t}\n1%\n"));
    }
    for _ in 0..shape.transactions {
        t += 5;
        out.push_str(&format!("#{t}\n0#\n")); // CS low: transaction starts
        if shape.command_phase {
            t += 1;
            out.push_str(&format!("#{t}\n0$\n")); // D/C low: command phase
            for _ in 0..8 {
                t += 1;
                out.push_str(&format!("#{t}\n1!\n1\"\n"));
                t += 1;
                out.push_str(&format!("#{t}\n0!\n0\"\n"));
            }
        }
        t += 1;
        out.push_str(&format!("#{t}\n1$\n")); // D/C high: data phase
        for i in 0..shape.bits {
            t += 1;
            let mosi = u8::from(i % 3 == 0);
            out.push_str(&format!("#{t}\n1!\n{mosi}\"\n"));
            t += 1;
            out.push_str(&format!("#{t}\n0!\n"));
        }
        t += 1;
        out.push_str(&format!("#{t}\n1#\n")); // CS high: transaction ends
    }
    out
}

/// Thresholds small enough for a synthetic capture, but still non-zero.
fn small() -> Thresholds {
    Thresholds {
        min_clock_edges: 100,
        min_cs_transitions: 8,
        min_dc_transitions: 6,
    }
}

fn finding(findings: &[kivori_wokwi_vcd::Finding], name: &str) -> bool {
    findings
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no finding named {name}"))
        .ok
}

#[test]
fn a_well_formed_capture_parses_with_every_signal() {
    let vcd = parse(&capture(&Shape::default())).expect("parse");
    assert_eq!(vcd.signals.len(), 5);
    assert_eq!(vcd.timescale.as_deref(), Some("1us"));
    for role in Role::ALL {
        assert!(vcd.resolve(role).is_ok(), "{} must resolve", role.name());
    }
    assert!(!vcd.edges(Role::Sck).expect("sck").is_empty());
}

#[test]
fn a_file_with_no_var_declarations_is_rejected() {
    assert_eq!(parse("garbage\n#0\n1!\n").err(), Some(VcdError::NoSignals));
    assert_eq!(parse("").err(), Some(VcdError::NoSignals));
}

#[test]
fn a_truncated_header_is_rejected() {
    let text = "$timescale 1us $end\n$var wire 1 ! lcd:SCK $end\n";
    assert_eq!(parse(text).err(), Some(VcdError::TruncatedHeader));
}

#[test]
fn unknown_signal_names_fail_loudly_and_list_what_is_present() {
    let text = "$timescale 1us $end\n\
                $var wire 1 ! mystery_net_a $end\n\
                $var wire 1 \" mystery_net_b $end\n\
                $enddefinitions $end\n#0\n1!\n";
    let vcd = parse(text).expect("parse");
    let err = vcd.resolve(Role::Sck).expect_err("must not resolve");
    match err {
        VcdError::UnresolvedRole { role, available } => {
            assert_eq!(role, Role::Sck);
            assert_eq!(available, vec!["mystery_net_a", "mystery_net_b"]);
        }
        other => panic!("wrong error: {other:?}"),
    }
    // And the whole check must fail, not silently skip the role.
    assert!(check(&vcd, small()).is_err());
}

#[test]
fn every_property_holds_for_a_good_capture() {
    let vcd = parse(&capture(&Shape::default())).expect("parse");
    let findings = check(&vcd, small()).expect("check");
    for f in &findings {
        assert!(f.ok, "{} should hold: {}", f.name, f.detail);
    }
    assert_eq!(findings.len(), 8, "all eight properties are reported");
}

#[test]
fn a_silent_clock_fails_activity_and_volume() {
    let shape = Shape {
        bits: 0,
        command_phase: false,
        ..Shape::default()
    };
    let vcd = parse(&capture(&shape)).expect("parse");
    let findings = check(&vcd, small()).expect("check");
    assert!(!finding(&findings, "clock-activity"));
    assert!(!finding(&findings, "transfer-volume"));
    // CS still framed transactions, so that property must remain independent.
    assert!(finding(&findings, "cs-transitions"));
}

#[test]
fn data_without_a_preceding_command_phase_fails() {
    let shape = Shape {
        command_phase: false,
        ..Shape::default()
    };
    let vcd = parse(&capture(&shape)).expect("parse");
    let findings = check(&vcd, small()).expect("check");
    assert!(
        !finding(&findings, "data-follows-command"),
        "clocking data with no command phase must fail"
    );
    assert!(
        finding(&findings, "clock-activity"),
        "the bus still clocked"
    );
}

#[test]
fn a_missing_reset_pulse_fails_reset_and_ordering() {
    let shape = Shape {
        reset: false,
        ..Shape::default()
    };
    let vcd = parse(&capture(&shape)).expect("parse");
    let findings = check(&vcd, small()).expect("check");
    assert!(!finding(&findings, "reset-sequence"));
    assert!(!finding(&findings, "stage-ordering"));
}

#[test]
fn too_few_framed_transactions_fail_the_cs_and_dc_gates() {
    let shape = Shape {
        transactions: 1,
        ..Shape::default()
    };
    let vcd = parse(&capture(&shape)).expect("parse");
    let findings = check(&vcd, small()).expect("check");
    assert!(!finding(&findings, "cs-transitions"));
    assert!(!finding(&findings, "dc-transitions"));
}

#[test]
fn the_default_transfer_volume_gate_rejects_a_thin_capture() {
    // The default threshold is a real gate: a synthetic capture nowhere near a tile's worth of bits
    // must not pass it.
    let vcd = parse(&capture(&Shape::default())).expect("parse");
    let findings = check(&vcd, Thresholds::default()).expect("check");
    assert!(
        !finding(&findings, "transfer-volume"),
        "the default volume gate must bite"
    );
}

#[test]
fn unknown_and_high_impedance_levels_count_as_low() {
    let text = format!("{}#0\nx!\n#1\nz!\n#2\n1!\n#3\n0!\n", header());
    let vcd = parse(&text).expect("parse");
    let edges = vcd.edges(Role::Sck).expect("sck");
    assert_eq!(
        edges,
        &[
            Transition {
                time: 0,
                level: false
            },
            Transition {
                time: 1,
                level: false
            },
            Transition {
                time: 2,
                level: true
            },
            Transition {
                time: 3,
                level: false
            },
        ]
    );
    assert_eq!(rising_edges(edges), 1);
    assert_eq!(transitions(edges), 2);
}

#[test]
fn vector_value_changes_are_parsed() {
    let text = format!("{}#0\nb0000 !\n#5\nb0001 !\n", header());
    let vcd = parse(&text).expect("parse");
    let edges = vcd.edges(Role::Sck).expect("sck");
    assert_eq!(edges.len(), 2);
    assert!(!edges[0].level);
    assert!(edges[1].level);
}

#[test]
fn edge_counters_ignore_repeated_levels() {
    let edges = [
        Transition {
            time: 0,
            level: true,
        },
        Transition {
            time: 1,
            level: true,
        },
        Transition {
            time: 2,
            level: false,
        },
        Transition {
            time: 3,
            level: true,
        },
    ];
    assert_eq!(rising_edges(&edges), 2);
    assert_eq!(transitions(&edges), 2);
}

#[test]
fn a_floating_high_sample_before_reset_is_not_mistaken_for_data() {
    // Regression, taken from a REAL capture: Wokwi records all five lines as `1` at time 0, because the
    // pins float until the firmware configures them ~164 ms into boot. Measuring from time 0 made
    // `data-follows-command` fail on a perfectly good bus, since a "data-phase clock" appeared at t=0
    // before any command had been sent.
    let shape = Shape {
        floating_prefix: true,
        offset: 164_000_000,
        ..Shape::default()
    };
    let vcd = parse(&capture(&shape)).expect("parse");
    let findings = check(&vcd, small()).expect("check");
    for f in &findings {
        assert!(
            f.ok,
            "{} should hold on a real-shaped capture: {}",
            f.name, f.detail
        );
    }
}

#[test]
fn the_post_reset_window_is_what_makes_the_difference() {
    // Non-vacuity control for the fix: measured from 0 the floating sample DOES look like a data clock;
    // measured from the reset release it does not.
    let shape = Shape {
        floating_prefix: true,
        offset: 164_000_000,
        ..Shape::default()
    };
    let vcd = parse(&capture(&shape)).expect("parse");
    let sck = vcd.edges(Role::Sck).expect("sck");
    let dc = vcd.edges(Role::Dc).expect("dc");
    let reset = vcd.edges(Role::Reset).expect("rst");
    let (_, released) = kivori_wokwi_vcd::pulse_low_then_high(reset).expect("reset pulse");

    assert_eq!(
        first_clock_while(sck, dc, true, 0),
        Some(0),
        "from time 0 the floating-high sample looks like a data-phase clock"
    );
    let windowed = first_clock_while(sck, dc, true, released).expect("a real data clock");
    assert!(
        windowed >= released,
        "inside the post-reset window the first data clock must be real traffic"
    );
    // And the pre-reset noise must be excluded from the activity counts.
    assert!(since(sck, released).len() < sck.len());
}
