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

/// Two-line stacks drawn after the power figure in the status bar.
pub fn status_bar_stacks(mode: StatusBarSystem, stats: Option<&SystemStats>) -> Vec<[String; 2]> {
    let Some(stats) = stats else {
        return Vec::new();
    };
    let pct = |v: f32| format!("{:.0}%", v.clamp(0.0, 100.0));
    let mem = if stats.memory.total > 0 {
        stats.memory.used as f32 / stats.memory.total as f32 * 100.0
    } else {
        0.0
    };
    let net = || {
        [
            format!("↓{}", short_rate(stats.network.down_rate)),
            format!("↑{}", short_rate(stats.network.up_rate)),
        ]
    };
    match mode {
        StatusBarSystem::None => Vec::new(),
        StatusBarSystem::Cpu => vec![["CPU".into(), pct(stats.cpu.usage)]],
        StatusBarSystem::Network => vec![net()],
        StatusBarSystem::Compact => {
            let mut out = vec![[
                format!("C {}", pct(stats.cpu.usage)),
                format!("M {}", pct(mem)),
            ]];
            if let Some(gpu) = stats.gpu {
                out[0] = [
                    format!("C {}", pct(stats.cpu.usage)),
                    format!("G {}", pct(gpu.usage)),
                ];
                out.push([format!("M {}", pct(mem)), net()[0].clone()]);
            } else {
                out.push(net());
            }
            out
        }
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
    fn stacks_follow_mode() {
        let mut stats = SystemStats::default();
        stats.cpu.usage = 23.4;
        stats.memory.total = 100;
        stats.memory.used = 61;
        stats.network.down_rate = 1.25 * 1024.0 * 1024.0;
        stats.network.up_rate = 86.0 * 1024.0;
        assert!(status_bar_stacks(StatusBarSystem::None, Some(&stats)).is_empty());
        assert!(status_bar_stacks(StatusBarSystem::Cpu, None).is_empty());
        assert_eq!(
            status_bar_stacks(StatusBarSystem::Cpu, Some(&stats)),
            vec![["CPU".to_string(), "23%".to_string()]]
        );
        assert_eq!(
            status_bar_stacks(StatusBarSystem::Network, Some(&stats)),
            vec![["↓1.2M".to_string(), "↑86K".to_string()]]
        );
        // No GPU: CPU/memory, then network.
        assert_eq!(
            status_bar_stacks(StatusBarSystem::Compact, Some(&stats)),
            vec![
                ["C 23%".to_string(), "M 61%".to_string()],
                ["↓1.2M".to_string(), "↑86K".to_string()],
            ]
        );
        stats.gpu = Some(tpower::system::GpuStats {
            usage: 8.0,
            memory_used: 0,
        });
        assert_eq!(
            status_bar_stacks(StatusBarSystem::Compact, Some(&stats)),
            vec![
                ["C 23%".to_string(), "G 8%".to_string()],
                ["M 61%".to_string(), "↓1.2M".to_string()],
            ]
        );
    }
}
