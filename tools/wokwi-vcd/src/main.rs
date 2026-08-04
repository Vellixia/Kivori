//! Checks a Wokwi VCD capture from the generic SPI display probe.
//!
//! ```text
//! cargo run -q -p kivori-wokwi-vcd -- target/wokwi-vcd/spi-display-probe.vcd
//! ```
//!
//! Exit codes: `0` every property held, `1` a property failed, `2` the file is unusable (missing,
//! unparseable, or its signals could not be identified). An unresolved capture is a FAILURE — the checker
//! never passes merely because a file exists.
//!
//! Optional flags:
//!   `--min-clock-edges <n>`   override the minimum clocked-bit count
//!   `--list-signals`          print every declared signal name and exit 0 (for adapting role candidates
//!                             from a real capture)

use kivori_wokwi_vcd::{check, parse, Thresholds};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut path: Option<String> = None;
    let mut thresholds = Thresholds::default();
    let mut list_only = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--list-signals" => list_only = true,
            "--min-clock-edges" => {
                let Some(value) = args.next().and_then(|v| v.parse::<usize>().ok()) else {
                    eprintln!("error: --min-clock-edges needs a number");
                    return ExitCode::from(2);
                };
                thresholds.min_clock_edges = value;
            }
            other => path = Some(other.to_string()),
        }
    }

    let Some(path) = path else {
        eprintln!("usage: kivori-wokwi-vcd [--list-signals] [--min-clock-edges N] <capture.vcd>");
        return ExitCode::from(2);
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("error: cannot read {path}: {err}");
            return ExitCode::from(2);
        }
    };

    let vcd = match parse(&text) {
        Ok(vcd) => vcd,
        Err(err) => {
            eprintln!("error: {path}: {err}");
            return ExitCode::from(2);
        }
    };

    println!(
        "VCD {path}: {} signals, timescale {}",
        vcd.signals.len(),
        vcd.timescale.as_deref().unwrap_or("unspecified")
    );
    if list_only {
        for name in &vcd.signals {
            println!("  signal: {name}");
        }
        return ExitCode::SUCCESS;
    }

    let findings = match check(&vcd, thresholds) {
        Ok(findings) => findings,
        Err(err) => {
            eprintln!("error: {path}: {err}");
            eprintln!(
                "hint: re-run with --list-signals and adapt Role::candidates from the real names."
            );
            return ExitCode::from(2);
        }
    };

    let mut failed = 0;
    for finding in &findings {
        let mark = if finding.ok { "✓" } else { "✗" };
        println!("  {mark} {}: {}", finding.name, finding.detail);
        if !finding.ok {
            failed += 1;
        }
    }

    if failed > 0 {
        eprintln!(
            "VCD check FAILED: {failed} of {} properties",
            findings.len()
        );
        return ExitCode::from(1);
    }
    println!("VCD check OK: {} digital properties held.", findings.len());
    println!("NOTE: a VCD proves digital events only — no analog integrity, no timing margin, and nothing");
    println!("      about any real display controller.");
    ExitCode::SUCCESS
}
