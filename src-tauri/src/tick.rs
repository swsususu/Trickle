//! Delivery of power and system samples to the web views.
//!
//! Every event emitted to a web view is an `evaluateJavaScript`, and WebKit
//! has to resume a suspended WebContent process to run it. The status bar
//! panel is closed nearly all the time, yet it used to receive every tick, so
//! each tick woke the process, failed to mark its layers volatile, and
//! suspended it again: ~70 lines of WebKit logging and a CPU wakeup per tick
//! for nothing on screen.
//!
//! Samples now go only to a web view that is on screen. While it is not, they
//! are queued here (bounded) and sent in one batch when it reappears, so its
//! charts still cover the time it was hidden.
//!
//! Each window listens on its own event name (`tick:<label>`): Tauri evaluates
//! an event in every web view that has a listener for that name, whatever the
//! emit target, so a shared name would wake all of them again.

use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
};

use objc::{msg_send, sel, sel_impl};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_nspopover::AppExt;
use tpower::{provider::NormalizedResource, system::SystemStats};

/// Matches the longest history the frontend keeps (60 system samples).
const BACKLOG_LIMIT: usize = 60;
/// Web views that render samples. Settings does not.
const TARGETS: [&str; 2] = ["main", "popover"];
/// `NSWindowOcclusionStateVisible`.
const NS_WINDOW_OCCLUSION_STATE_VISIBLE: usize = 1 << 1;

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
#[serde(rename_all = "camelCase")]
pub struct TickSample {
    /// Unix time in milliseconds when the sample was taken.
    pub time: i64,
    /// `None` when the IORegistry read failed.
    pub power: Option<NormalizedResource>,
    /// `None` when system monitoring is switched off.
    pub system: Option<SystemStats>,
}

/// Payload of the `tick:<label>` event: queued samples, oldest first. The
/// last one is the current reading.
#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct TickBatch {
    pub samples: Vec<TickSample>,
}

#[derive(Default)]
pub struct TickBacklog(Mutex<HashMap<&'static str, VecDeque<TickSample>>>);

/// Must run on the main thread: it queries AppKit.
fn is_on_screen<R: Runtime>(app: &AppHandle<R>, label: &str) -> bool {
    if label == "popover" {
        return app.is_popover_shown();
    }
    let Some(ns_window) = app
        .get_webview_window(label)
        .and_then(|window| window.ns_window().ok())
    else {
        return false;
    };
    // Covers hidden, minimised, and fully covered windows alike: WebKit
    // treats all of them as not visible. AppKit updates the occlusion state
    // asynchronously, so a window that has just been shown and focused may
    // not report it yet; being the key window is enough then.
    let window = ns_window as *mut objc::runtime::Object;
    let state: usize = unsafe { msg_send![window, occlusionState] };
    let key: bool = unsafe { msg_send![window, isKeyWindow] };
    state & NS_WINDOW_OCCLUSION_STATE_VISIBLE != 0 || key
}

/// Must run on the main thread.
fn deliver_on_main<R: Runtime>(app: &AppHandle<R>, sample: Option<TickSample>) {
    let Some(state) = app.try_state::<TickBacklog>() else {
        return;
    };
    let mut backlog = state.0.lock().unwrap();
    for label in TARGETS {
        let queue = backlog.entry(label).or_default();
        if let Some(sample) = &sample {
            queue.push_back(sample.clone());
            if queue.len() > BACKLOG_LIMIT {
                queue.pop_front();
            }
        }
        if queue.is_empty() || !is_on_screen(app, label) {
            continue;
        }
        let batch = TickBatch {
            samples: queue.drain(..).collect(),
        };
        if let Err(error) = app.emit(&format!("tick:{label}"), batch) {
            log::error!("Failed to emit tick to {label}: {error}");
        }
    }
}

/// Queue a new sample and send it to every web view that is on screen.
pub fn deliver<R: Runtime>(app: &AppHandle<R>, sample: TickSample) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || deliver_on_main(&handle, Some(sample)));
}

/// Send the queued samples to any web view that has just come on screen,
/// without waiting for the next tick.
pub fn flush<R: Runtime>(app: &AppHandle<R>) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || deliver_on_main(&handle, None));
}
