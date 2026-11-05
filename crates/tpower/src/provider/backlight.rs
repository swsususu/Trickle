//! Built-in display backlight power estimate for Macs without an SMC
//! backlight rail.
//!
//! Only MacBook Pro mini-LED panels (`PDBR`, M1-M4) and M5 / A18 machines
//! (`PBwo`) report backlight power through the SMC. Everything else, e.g. the
//! MacBook Air line, exposes nothing there. `AppleARMBacklight` still publishes
//! the live LED drive current, so we estimate power from it, the same way
//! macpow does (https://github.com/k06a/macpow):
//!
//!   watts = BrightnessMicroAmps.value / BrightnessMicroAmps.max
//!         * current-for-max-backlight (mA) * LED string voltage
//!
//! The LED string voltage is not exposed by macOS, so this is an estimate and
//! is flagged as such to the UI.

use std::{ffi::CString, mem};

use core_foundation::{
    base::{kCFAllocatorDefault, CFType, TCFType},
    boolean::CFBoolean,
    dictionary::{CFDictionary, CFMutableDictionaryRef},
    string::CFString,
};
use io_kit_sys::{
    kIOMasterPortDefault, ret::kIOReturnSuccess, IOObjectRelease, IORegistryEntryCreateCFProperties,
    IORegistryEntryCreateCFProperty, IOServiceGetMatchingService, IOServiceMatching,
};
use serde::Deserialize;

use crate::util::dict_into;

/// Assumed backlight LED string voltage (not exposed by macOS).
const BACKLIGHT_VOLTAGE_V: f32 = 6.0;
/// Upper bound for a built-in panel; anything above is a misread.
const MAX_PLAUSIBLE_W: f32 = 15.0;

#[derive(Debug, Default, Deserialize)]
struct Range {
    #[serde(default)]
    value: i64,
    #[serde(default)]
    max: i64,
}

#[derive(Debug, Default, Deserialize)]
struct DisplayParameters {
    #[serde(rename = "BrightnessMicroAmps")]
    micro_amps: Option<Range>,
}

#[derive(Debug, Default, Deserialize)]
struct Calibration {
    /// Drive current at full brightness, in µA.
    #[serde(rename = "current-for-max-backlight")]
    max_current_ua: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
struct BacklightProperties {
    #[serde(rename = "IODisplayParameters")]
    params: Option<DisplayParameters>,
    #[serde(rename = "backlight-calibration-parameters")]
    calibration: Option<Calibration>,
}

/// Pure part of the estimate, split out for testing.
fn watts_from(props: &BacklightProperties) -> Option<f32> {
    let amps = props.params.as_ref()?.micro_amps.as_ref()?;
    let max_current_ua = props.calibration.as_ref()?.max_current_ua?;
    if amps.max <= 0 || max_current_ua <= 0 {
        return None;
    }
    let fraction = (amps.value.max(0) as f32 / amps.max as f32).min(1.0);
    let watts = fraction * (max_current_ua as f32 / 1_000_000.0) * BACKLIGHT_VOLTAGE_V;
    (watts.is_finite() && watts <= MAX_PLAUSIBLE_W).then_some(watts)
}

fn service_properties(class: &str) -> Option<CFDictionary> {
    let name = CString::new(class).ok()?;
    unsafe {
        let service =
            IOServiceGetMatchingService(kIOMasterPortDefault, IOServiceMatching(name.as_ptr()));
        if service == 0 {
            return None;
        }
        let mut props: CFMutableDictionaryRef = mem::zeroed();
        let kr = IORegistryEntryCreateCFProperties(service, &mut props, kCFAllocatorDefault, 0);
        IOObjectRelease(service);
        if kr != kIOReturnSuccess || props.is_null() {
            return None;
        }
        Some(CFDictionary::wrap_under_create_rule(props))
    }
}

/// True when the lid is closed. The panel is off then, but
/// `BrightnessMicroAmps` keeps its last value.
fn clamshell_closed() -> bool {
    let name = CString::new("IOPMrootDomain").unwrap();
    unsafe {
        let service =
            IOServiceGetMatchingService(kIOMasterPortDefault, IOServiceMatching(name.as_ptr()));
        if service == 0 {
            return false;
        }
        let key = CFString::from_static_string("AppleClamshellState");
        let value = IORegistryEntryCreateCFProperty(
            service,
            key.as_concrete_TypeRef(),
            kCFAllocatorDefault,
            0,
        );
        IOObjectRelease(service);
        if value.is_null() {
            return false;
        }
        CFType::wrap_under_create_rule(value)
            .downcast::<CFBoolean>()
            .is_some_and(bool::from)
    }
}

/// Estimated built-in backlight power in watts, or `None` when this Mac does
/// not publish the backlight drive current.
pub fn estimate_power() -> Option<f32> {
    let props = dict_into::<BacklightProperties>(service_properties("AppleARMBacklight")?).ok()?;
    let watts = watts_from(&props)?;
    Some(if clamshell_closed() { 0.0 } else { watts })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(value: i64, max: i64, max_current_ua: Option<i64>) -> BacklightProperties {
        BacklightProperties {
            params: Some(DisplayParameters {
                micro_amps: Some(Range { value, max }),
            }),
            calibration: Some(Calibration { max_current_ua }),
        }
    }

    #[test]
    fn estimates_from_drive_current() {
        // Values read from a 14" MacBook Pro at 50% brightness.
        let w = watts_from(&props(3199, 13799, Some(904_383))).unwrap();
        assert!((w - 1.258).abs() < 0.01, "{w}");
    }

    #[test]
    fn full_brightness_hits_calibrated_max() {
        let w = watts_from(&props(13799, 13799, Some(904_383))).unwrap();
        assert!((w - 5.426).abs() < 0.01, "{w}");
    }

    #[test]
    fn missing_fields_yield_none() {
        assert!(watts_from(&BacklightProperties::default()).is_none());
        assert!(watts_from(&props(100, 0, Some(904_383))).is_none());
        assert!(watts_from(&props(100, 1000, None)).is_none());
    }

    #[test]
    fn implausible_values_are_rejected() {
        assert!(watts_from(&props(1, 1, Some(10_000_000))).is_none());
    }
}
