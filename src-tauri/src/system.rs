//! System load monitoring glued onto the power tick.
//!
//! Sampling shares the power-tick timer instead of running its own: every
//! source is a kernel counter read costing well under a millisecond, so a
//! second timer would only add wakeups.

use std::fmt;

use serde::{de, Deserialize, Deserializer, Serialize};
use specta::Type;
use tauri_specta::Event;
use tpower::system::{SystemSampler, SystemStats};

#[derive(Serialize, Deserialize, Debug, Clone, Event, Type)]
#[serde(rename_all = "camelCase")]
pub struct SystemTickEvent {
    pub data: SystemStats,
}

/// What, besides power, the status bar title shows.
///
/// Deserialisation is forgiving for the same reason as `StatusBarItem`: a bad
/// persisted value must not panic the power-tick task.
#[derive(Serialize, Debug, Clone, Copy, Default, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub enum StatusBarSystem {
    #[default]
    None,
    Cpu,
    Network,
    Compact,
}

impl<'de> Deserialize<'de> for StatusBarSystem {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl de::Visitor<'_> for V {
            type Value = StatusBarSystem;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a status bar system item string")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<StatusBarSystem, E> {
                Ok(match v {
                    "cpu" => StatusBarSystem::Cpu,
                    "network" => StatusBarSystem::Network,
                    "compact" => StatusBarSystem::Compact,
                    _ => StatusBarSystem::None,
                })
            }
            fn visit_unit<E: de::Error>(self) -> Result<StatusBarSystem, E> {
                Ok(StatusBarSystem::None)
            }
            fn visit_none<E: de::Error>(self) -> Result<StatusBarSystem, E> {
                Ok(StatusBarSystem::None)
            }
            fn visit_bool<E: de::Error>(self, _: bool) -> Result<StatusBarSystem, E> {
                Ok(StatusBarSystem::None)
            }
        }
        deserializer.deserialize_any(V)
    }
}

pub struct SystemMonitor {
    sampler: SystemSampler,
    pub enabled: bool,
    pub status_bar: StatusBarSystem,
}

impl SystemMonitor {
    pub fn new(enabled: bool, status_bar: StatusBarSystem) -> Self {
        Self {
            sampler: SystemSampler::new(),
            enabled,
            status_bar,
        }
    }

    /// `None` when monitoring is switched off, so nothing is read at all.
    pub fn sample(&mut self) -> Option<SystemStats> {
        self.enabled.then(|| self.sampler.sample())
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if enabled && !self.enabled {
            // Drop stale counters so the first rate after re-enabling is not
            // averaged over the whole time monitoring was off.
            self.sampler = SystemSampler::new();
        }
        self.enabled = enabled;
    }
}

/// Short, fixed-ish width rate for the status bar: `860K`, `1.2M`, `12M`.
pub fn short_rate(bytes_per_sec: f64) -> String {
    let kb = bytes_per_sec / 1024.0;
    if kb < 1000.0 {
        format!("{:.0}K", kb)
    } else if kb < 10.0 * 1024.0 {
        format!("{:.1}M", kb / 1024.0)
    } else {
        format!("{:.0}M", kb / 1024.0)
    }
}

/// Text appended after the power figure in the status bar title.
pub fn status_bar_suffix(mode: StatusBarSystem, stats: Option<&SystemStats>) -> Option<String> {
    let stats = stats?;
    let mem = if stats.memory.total > 0 {
        stats.memory.used as f64 / stats.memory.total as f64 * 100.0
    } else {
        0.0
    };
    match mode {
        StatusBarSystem::None => None,
        StatusBarSystem::Cpu => Some(format!("{:.0}%", stats.cpu.usage)),
        StatusBarSystem::Network => Some(format!(
            "↓{} ↑{}",
            short_rate(stats.network.down_rate),
            short_rate(stats.network.up_rate)
        )),
        StatusBarSystem::Compact => Some(format!(
            "C{:.0}% G{:.0}% M{:.0}%",
            stats.cpu.usage,
            stats.gpu.map_or(0.0, |g| g.usage),
            mem
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_rate_formats() {
        assert_eq!(short_rate(0.0), "0K");
        assert_eq!(short_rate(86.0 * 1024.0), "86K");
        assert_eq!(short_rate(1.25 * 1024.0 * 1024.0), "1.2M");
        assert_eq!(short_rate(48.0 * 1024.0 * 1024.0), "48M");
    }

    #[test]
    fn status_bar_system_tolerates_bad_values() {
        for (json, want) in [
            ("\"cpu\"", StatusBarSystem::Cpu),
            ("\"compact\"", StatusBarSystem::Compact),
            ("\"bogus\"", StatusBarSystem::None),
            ("null", StatusBarSystem::None),
            ("true", StatusBarSystem::None),
        ] {
            assert_eq!(serde_json::from_str::<StatusBarSystem>(json).unwrap(), want);
        }
    }

    #[test]
    fn suffix_follows_mode() {
        let mut stats = SystemStats::default();
        stats.cpu.usage = 23.4;
        stats.memory.total = 100;
        stats.memory.used = 61;
        assert_eq!(status_bar_suffix(StatusBarSystem::None, Some(&stats)), None);
        assert_eq!(status_bar_suffix(StatusBarSystem::Cpu, None), None);
        assert_eq!(
            status_bar_suffix(StatusBarSystem::Cpu, Some(&stats)).as_deref(),
            Some("23%")
        );
        assert_eq!(
            status_bar_suffix(StatusBarSystem::Compact, Some(&stats)).as_deref(),
            Some("C23% G0% M61%")
        );
    }
}
