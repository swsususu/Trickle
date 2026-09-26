//! Prints two system load samples one second apart.
//!
//! Run with: cargo run -p tpower --example system

use std::{thread, time::Duration};

use tpower::system::SystemSampler;

fn main() {
    let mut sampler = SystemSampler::new();
    sampler.sample();
    thread::sleep(Duration::from_secs(1));
    println!("{:#?}", sampler.sample());
}
