//! Diagnostic probe: reads the real IORegistry + SMC data through the same
//! code path the app uses, and prints the normalized result.
//!
//! Run with: cargo run -p tpower --example probe

use tpower::{
    ffi::smc::{SMCConnection, SMCReadSensor},
    provider::{get_mac_ioreg, NormalizedResource},
};

fn main() {
    let io = match get_mac_ioreg() {
        Ok(io) => io,
        Err(e) => {
            eprintln!("get_mac_ioreg failed: {e}");
            return;
        }
    };

    println!("=== raw IORegistry ===");
    println!("current_capacity        = {}", io.current_capacity);
    println!("max_capacity            = {}", io.max_capacity);
    println!("design_capacity         = {}", io.design_capacity);
    println!("apple_raw_current_cap   = {}", io.apple_raw_current_capacity);
    println!("apple_raw_max_capacity  = {}", io.apple_raw_max_capacity);
    println!("time_remaining          = {}", io.time_remaining);
    println!("temperature             = {}", io.temperature);
    println!("cycle_count             = {}", io.cycle_count);
    println!("is_charging             = {}", io.is_charging);
    println!("amperage                = {}", io.amperage);
    println!("absolute_capacity       = {}", io.absolute_capacity);
    println!("battery_data            = {:?}", io.battery_data);
    println!("adapter_details.name    = {:?}", io.adapter_details.name);

    let mut smc = match SMCConnection::new("AppleSMC") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\nSMCConnection failed: {e} (app degrades gracefully here)");
            return;
        }
    };
    let smc_data = smc.read_sensor();
    println!("\n=== SMC ===");
    println!("{smc_data:#?}");

    let n = NormalizedResource::from((&io, &smc_data));
    println!("\n=== normalized (what the UI shows) ===");
    println!("is_charging          = {}", n.is_charging);
    println!("time_remain          = {:?}", n.time_remain);
    println!("time_remain_known    = {}", n.time_remain_known);
    println!("cycle_count          = {}", n.cycle_count);
    println!("current_capacity mAh = {}", n.current_capacity);
    println!("max_capacity     mAh = {}", n.max_capacity);
    println!("design_capacity  mAh = {}", n.design_capacity);
    if n.design_capacity > 0 {
        println!(
            "battery_health       = {:.1}%",
            n.max_capacity as f32 / n.design_capacity as f32 * 100.
        );
    } else {
        println!("battery_health       = N/A (design_capacity is 0)");
    }
    println!("adapter_name         = {:?}", n.adapter_name);
    println!("battery_level        = {}", n.data.battery_level);
    println!("absolute_level       = {:.1}%", n.data.absolute_battery_level);
    println!("temperature          = {:.2}", n.data.temperature);
    println!("system_in            = {:.2} W", n.data.system_in);
    println!("system_load          = {:.2} W", n.data.system_load);
    println!("battery_power        = {:.2} W", n.data.battery_power);
    println!("brightness_power     = {:.2} W", n.data.brightness_power);
    println!("heatpipe_power       = {:.2} W", n.data.heatpipe_power);
    println!("adapter_power        = {:.2} W", n.data.adapter_power);
    println!("efficiency_loss      = {:.2} W", n.data.efficiency_loss);
}
