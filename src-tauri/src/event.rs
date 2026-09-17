use std::fmt;

use serde::{de, Deserialize, Deserializer, Serialize};
use specta::Type;
use tauri_specta::Event;
use tpower::ffi::{Action, InterfaceType};

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    Light,
    Dark,
    System,
}

/// Which power metric to show in the status bar.
///
/// Implements a forgiving `Deserialize`: unknown or non-string values (e.g. a
/// stale `"none"` or `null` persisted by older builds) fall back to `System`
/// instead of panicking inside tauri-specta and killing the power-tick task.
#[derive(Serialize, Debug, Clone, Default, Type)]
#[serde(rename_all = "camelCase")]
pub enum StatusBarItem {
    #[default]
    System,
    Screen,
    Heatpipe,
}

impl<'de> Deserialize<'de> for StatusBarItem {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> de::Visitor<'de> for V {
            type Value = StatusBarItem;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a status bar item string")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<StatusBarItem, E> {
                match v {
                    "system" => Ok(StatusBarItem::System),
                    "screen" => Ok(StatusBarItem::Screen),
                    "heatpipe" => Ok(StatusBarItem::Heatpipe),
                    other => {
                        log::warn!("Unknown StatusBarItem '{other}', falling back to System");
                        Ok(StatusBarItem::System)
                    }
                }
            }
            // A malformed preference file must not be fatal either: the whole
            // point of this impl is that the sampling loop keeps running.
            fn visit_unit<E: de::Error>(self) -> Result<StatusBarItem, E> {
                log::warn!("StatusBarItem was null, falling back to System");
                Ok(StatusBarItem::System)
            }
            fn visit_none<E: de::Error>(self) -> Result<StatusBarItem, E> {
                self.visit_unit()
            }
        }
        // `deserialize_any` lets the visitor see null and other value kinds,
        // where `deserialize_str` would error out before reaching it.
        deserializer.deserialize_any(V)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Event, Type)]
#[serde(rename_all = "camelCase")]
pub enum PreferenceEvent {
    Theme(Theme),
    AnimationsEnabled(bool),
    UpdateInterval(u32),
    Language(String),
    StatusBarItem(StatusBarItem),
    StatusBarShowCharging(bool),
}

#[derive(Serialize, Deserialize, Debug, Clone, Event, Type)]
#[serde(rename_all = "camelCase")]
pub struct DeviceEvent {
    pub udid: String,
    pub name: String,
    pub interface: InterfaceType,
    pub action: Action,
}

#[derive(Serialize, Deserialize, Debug, Clone, Event, Type)]
pub struct PowerUpdatedEvent(pub String);

#[derive(Serialize, Deserialize, Debug, Clone, Event, Type)]
pub struct WindowLoadedEvent;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_status_bar_items_round_trip() {
        for (json, expected) in [
            ("\"system\"", StatusBarItem::System),
            ("\"screen\"", StatusBarItem::Screen),
            ("\"heatpipe\"", StatusBarItem::Heatpipe),
        ] {
            let parsed: StatusBarItem = serde_json::from_str(json).unwrap();
            assert_eq!(
                std::mem::discriminant(&parsed),
                std::mem::discriminant(&expected),
                "{json} should parse to {expected:?}"
            );
        }
    }

    #[test]
    fn malformed_status_bar_items_fall_back_to_system() {
        // "none" was persisted by older builds; null can appear in a
        // truncated preference file. Neither may abort the power-tick task.
        for json in ["\"none\"", "\"\"", "null"] {
            let parsed: StatusBarItem = serde_json::from_str(json)
                .unwrap_or_else(|e| panic!("{json} must not fail to deserialize: {e}"));
            assert!(
                matches!(parsed, StatusBarItem::System),
                "{json} should fall back to System, got {parsed:?}"
            );
        }
    }
}
