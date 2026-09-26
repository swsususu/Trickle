//! Per-process energy usage, read from `top`.
//!
//! `powermetrics` gives richer data but needs root, which would mean shipping a
//! privileged helper. `top -stats power` exposes the same energy impact figure
//! the Activity Monitor "Energy" tab shows, with no elevation.
//!
//! This is deliberately on-demand rather than part of the power tick: `top`
//! needs two samples to produce an energy figure, so a call costs ~1.5s of
//! wall time. Polling it every couple of seconds would cost more energy than
//! the feature saves.

use std::process::Stdio;

use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::process::Command;

/// How many processes to report.
const TOP_N: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProcessEnergy {
    pub pid: i32,
    pub name: String,
    /// Energy impact, in the same arbitrary units Activity Monitor uses.
    /// Comparable between processes, not a wattage.
    pub impact: f32,
    /// CPU share from the same `top` sample, where 100 is one full core.
    pub cpu: f32,
    /// Memory footprint in bytes, as `top`'s MEM column reports it.
    pub memory: u64,
}

/// Reads the top energy consumers.
///
/// `-l 2` takes two samples because the first has no energy delta to report;
/// only the second frame is parsed. Returns an empty list rather than an error
/// when `top` is unavailable or its output is unparseable, since this is
/// supplementary information and must not surface as a failure.
pub async fn top_energy_processes() -> Vec<ProcessEnergy> {
    let output = Command::new("top")
        .args([
            "-l",
            "2",
            "-o",
            "power",
            "-n",
            // Ask for a few extra rows so filtering `top` itself out still
            // leaves TOP_N entries.
            &(TOP_N + 4).to_string(),
            "-stats",
            "pid,command,cpu,mem,power",
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await;

    let Ok(output) = output else {
        log::warn!("failed to run top for process energy");
        return Vec::new();
    };

    let text = String::from_utf8_lossy(&output.stdout);
    parse_top_output(&text)
}

/// Parses the last sample frame out of `top` output.
fn parse_top_output(text: &str) -> Vec<ProcessEnergy> {
    // Each sample frame restarts with a header row; take the final one so the
    // first (empty) energy frame is discarded.
    let Some(header_at) = text.rfind("PID") else {
        return Vec::new();
    };

    let mut rows = Vec::new();
    for line in text[header_at..].lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Layout is `<pid> <command...> <cpu> <mem> <power>`: the command can
        // contain spaces, so split off the ends and treat the middle as the
        // name.
        let mut parts = line.split_whitespace();
        let Some(pid) = parts.next().and_then(|v| v.parse::<i32>().ok()) else {
            continue;
        };
        let rest: Vec<&str> = parts.collect();
        if rest.len() < 4 {
            continue;
        }
        let n = rest.len();
        let (Ok(cpu), Some(memory), Ok(impact)) = (
            rest[n - 3].parse::<f32>(),
            parse_mem(rest[n - 2]),
            rest[n - 1].parse::<f32>(),
        ) else {
            continue;
        };
        let name = rest[..n - 3].join(" ");
        // `top` reports itself, which is noise caused by the measurement.
        if name == "top" {
            continue;
        }
        rows.push(ProcessEnergy {
            pid,
            name,
            impact,
            cpu,
            memory,
        });
    }

    rows.retain(|p| p.impact > 0.0);
    rows.truncate(TOP_N);
    rows
}

/// Parses `top`'s MEM column: `870M-`, `6160K+`, `1.2G`, `512B`.
///
/// The trailing `+`/`-` marks growth since the last sample and is dropped.
fn parse_mem(v: &str) -> Option<u64> {
    let v = v.trim_end_matches(['+', '-']);
    let (num, mul) = match v.chars().last()? {
        'B' => (&v[..v.len() - 1], 1u64),
        'K' => (&v[..v.len() - 1], 1 << 10),
        'M' => (&v[..v.len() - 1], 1 << 20),
        'G' => (&v[..v.len() - 1], 1 << 30),
        'T' => (&v[..v.len() - 1], 1 << 40),
        _ => (v, 1),
    };
    let n: f64 = num.parse().ok()?;
    Some((n * mul as f64) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_final_sample_frame() {
        // Two frames; only the second carries real numbers, matching how top
        // reports energy.
        let text = "\
Processes: 500 total
PID    COMMAND          %CPU MEM    POWER
1      launchd          0.0  12M    0.0
2      kernel_task      0.0  1G     0.0

Processes: 500 total
PID    COMMAND          %CPU MEM    POWER
608    WindowServer     35.2 310M+  37.1
57345  Code Helper (Ren 20.0 668M-  21.8
41538  top              5.2  6160K+ 5.2
";
        let rows = parse_top_output(text);
        assert_eq!(rows.len(), 2, "top itself and zero-impact rows are dropped");
        assert_eq!(rows[0].pid, 608);
        assert_eq!(rows[0].name, "WindowServer");
        assert!((rows[0].impact - 37.1).abs() < f32::EPSILON);
        // Command names containing spaces must survive intact.
        assert_eq!(rows[1].name, "Code Helper (Ren");
        assert!((rows[0].cpu - 35.2).abs() < f32::EPSILON);
        assert_eq!(rows[1].memory, 668 << 20);
    }

    #[test]
    fn parses_mem_column() {
        assert_eq!(parse_mem("6160K+"), Some(6160 << 10));
        assert_eq!(parse_mem("870M-"), Some(870 << 20));
        assert_eq!(parse_mem("1.5G"), Some(3 << 29));
        assert_eq!(parse_mem("512B"), Some(512));
        assert_eq!(parse_mem("N/A"), None);
    }

    #[test]
    fn tolerates_unusable_output() {
        assert!(parse_top_output("").is_empty());
        assert!(parse_top_output("no header here").is_empty());
        // A header with no rows is valid, just empty.
        assert!(parse_top_output("PID COMMAND %CPU MEM POWER\n").is_empty());
    }
}
