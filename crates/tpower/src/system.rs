//! System load sampling: CPU, GPU, memory and network throughput.
//!
//! Everything here reads kernel counters that need no privileges and have been
//! stable since at least macOS 11, so the same code runs on macOS 26 and 27 and
//! on Intel. Each source is independent: a missing one yields `None` or zeros
//! rather than failing the whole sample, because this data is supplementary to
//! the power readings and must never take them down.
//!
//! CPU and network figures are rates, so they are computed as deltas against
//! the previous call. The first sample after construction therefore reports
//! zero for them.

use std::{
    ffi::{c_void, CStr, CString},
    mem, ptr,
    time::Instant,
};

use core_foundation::{
    base::{kCFAllocatorDefault, CFType, TCFType},
    dictionary::CFDictionary,
    number::CFNumber,
    string::CFString,
};
use io_kit_sys::{
    keys::kIODeviceTreePlane, IOIteratorNext, IOMasterPort, IOObjectRelease,
    IORegistryEntryCreateCFProperty, IORegistryEntryFromPath, IORegistryEntryGetChildIterator,
    IOServiceGetMatchingServices, IOServiceMatching,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub cpu: CpuStats,
    /// `None` when no accelerator reports utilisation.
    pub gpu: Option<GpuStats>,
    pub memory: MemoryStats,
    pub network: NetworkStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CpuStats {
    /// Whole-machine busy share, 0-100.
    pub usage: f32,
    /// Per logical core, in kernel order.
    pub cores: Vec<CoreUsage>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CoreUsage {
    pub usage: f32,
    /// True for an efficiency core. Always false on Intel, which has none.
    pub efficiency: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GpuStats {
    /// 0-100.
    pub usage: f32,
    /// Bytes of memory the GPU driver has in use; 0 if not reported.
    pub memory_used: u64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum MemoryPressure {
    #[default]
    Normal,
    Warning,
    Critical,
}

impl MemoryPressure {
    /// Maps `kern.memorystatus_vm_pressure_level`, which uses the
    /// `DISPATCH_MEMORYPRESSURE_*` bit values (1, 2, 4).
    pub fn from_level(level: i32) -> Self {
        match level {
            4 => Self::Critical,
            2 => Self::Warning,
            _ => Self::Normal,
        }
    }
}

/// Sizes in bytes. `used` follows Activity Monitor: app + wired + compressed.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MemoryStats {
    pub total: u64,
    pub used: u64,
    pub app: u64,
    pub wired: u64,
    pub compressed: u64,
    pub swap_used: u64,
    pub pressure: MemoryPressure,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct NetworkStats {
    /// Bytes per second.
    pub down_rate: f64,
    pub up_rate: f64,
    /// Bytes since boot, over the counted interfaces.
    pub total_down: u64,
    pub total_up: u64,
    /// The busiest counted interface in this sample, e.g. `en0`.
    pub interface: Option<String>,
}

/// Keeps the previous counters needed to turn totals into rates.
pub struct SystemSampler {
    host: libc::mach_port_t,
    total_memory: u64,
    efficiency_cores: Vec<bool>,
    prev_ticks: Vec<[u32; 4]>,
    prev_net: Option<(Instant, Vec<InterfaceCounters>)>,
}

impl Default for SystemSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemSampler {
    pub fn new() -> Self {
        // `mach_host_self` hands out a send right each call; take it once.
        #[allow(deprecated)]
        let host = unsafe { libc::mach_host_self() };
        Self {
            host,
            total_memory: sysctl_u64("hw.memsize").unwrap_or(0),
            efficiency_cores: efficiency_core_map(),
            prev_ticks: Vec::new(),
            prev_net: None,
        }
    }

    pub fn sample(&mut self) -> SystemStats {
        SystemStats {
            cpu: self.sample_cpu(),
            gpu: sample_gpu(),
            memory: self.sample_memory(),
            network: self.sample_network(),
        }
    }

    fn sample_cpu(&mut self) -> CpuStats {
        let Some(ticks) = read_cpu_ticks(self.host) else {
            return CpuStats::default();
        };
        let prev = mem::replace(&mut self.prev_ticks, ticks);
        let (usage, per_core) = cpu_usage(&prev, &self.prev_ticks);
        CpuStats {
            usage,
            cores: per_core
                .into_iter()
                .enumerate()
                .map(|(i, usage)| CoreUsage {
                    usage,
                    efficiency: self.efficiency_cores.get(i).copied().unwrap_or(false),
                })
                .collect(),
        }
    }

    fn sample_memory(&self) -> MemoryStats {
        let mut stats = MemoryStats {
            total: self.total_memory,
            swap_used: swap_used().unwrap_or(0),
            pressure: MemoryPressure::from_level(
                sysctl_i32("kern.memorystatus_vm_pressure_level").unwrap_or(1),
            ),
            ..Default::default()
        };
        if let Some(vm) = read_vm_stats(self.host) {
            let page = page_size();
            let app =
                (vm.internal_page_count as u64).saturating_sub(vm.purgeable_count as u64) * page;
            let wired = vm.wire_count as u64 * page;
            let compressed = vm.compressor_page_count as u64 * page;
            stats.app = app;
            stats.wired = wired;
            stats.compressed = compressed;
            stats.used = (app + wired + compressed).min(self.total_memory.max(1));
        }
        stats
    }

    fn sample_network(&mut self) -> NetworkStats {
        let Some(now) = read_interface_counters() else {
            return NetworkStats::default();
        };
        let at = Instant::now();
        let counted: Vec<_> = now
            .into_iter()
            .filter(|c| is_counted_interface(&c.name))
            .collect();
        let mut stats = NetworkStats {
            total_down: counted.iter().map(|c| c.rx).sum(),
            total_up: counted.iter().map(|c| c.tx).sum(),
            ..Default::default()
        };
        if let Some((prev_at, prev)) = &self.prev_net {
            let secs = at.duration_since(*prev_at).as_secs_f64();
            let (down, up, busiest) = network_delta(prev, &counted);
            if secs > 0.0 {
                stats.down_rate = down as f64 / secs;
                stats.up_rate = up as f64 / secs;
            }
            stats.interface = busiest;
        }
        if stats.interface.is_none() {
            stats.interface = counted
                .iter()
                .max_by_key(|c| c.rx + c.tx)
                .map(|c| c.name.clone());
        }
        self.prev_net = Some((at, counted));
        stats
    }
}

// ---------------------------------------------------------------- CPU

/// user, system, idle, nice — the order of `CPU_STATE_*`.
fn read_cpu_ticks(host: libc::mach_port_t) -> Option<Vec<[u32; 4]>> {
    let mut count: libc::natural_t = 0;
    let mut info: libc::processor_info_array_t = ptr::null_mut();
    let mut info_count: libc::mach_msg_type_number_t = 0;
    let kr = unsafe {
        libc::host_processor_info(
            host,
            libc::PROCESSOR_CPU_LOAD_INFO,
            &mut count,
            &mut info,
            &mut info_count,
        )
    };
    if kr != libc::KERN_SUCCESS || info.is_null() {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(info, info_count as usize) };
    let ticks = slice
        .chunks_exact(libc::CPU_STATE_MAX as usize)
        .take(count as usize)
        .map(|c| [c[0] as u32, c[1] as u32, c[2] as u32, c[3] as u32])
        .collect();
    // The array is allocated in our address space by the kernel.
    #[allow(deprecated)]
    unsafe {
        libc::vm_deallocate(
            libc::mach_task_self(),
            info as libc::vm_address_t,
            info_count as usize * mem::size_of::<libc::integer_t>(),
        );
    }
    Some(ticks)
}

/// Busy share between two tick snapshots, overall and per core.
///
/// Tick counters are `u32` and wrap, so differences use wrapping arithmetic.
/// A core count change (or the first call, with no previous sample) yields
/// zeros rather than garbage.
pub fn cpu_usage(prev: &[[u32; 4]], now: &[[u32; 4]]) -> (f32, Vec<f32>) {
    if prev.len() != now.len() {
        return (0.0, vec![0.0; now.len()]);
    }
    let mut busy_sum = 0u64;
    let mut total_sum = 0u64;
    let per_core = prev
        .iter()
        .zip(now)
        .map(|(p, n)| {
            let d: [u64; 4] = std::array::from_fn(|i| n[i].wrapping_sub(p[i]) as u64);
            let busy = d[0] + d[1] + d[3];
            let total = busy + d[2];
            busy_sum += busy;
            total_sum += total;
            if total == 0 {
                0.0
            } else {
                busy as f32 / total as f32 * 100.0
            }
        })
        .collect();
    let overall = if total_sum == 0 {
        0.0
    } else {
        busy_sum as f32 / total_sum as f32 * 100.0
    };
    (overall, per_core)
}

/// Which logical CPUs are efficiency cores, indexed by logical CPU id.
///
/// Read from the device tree (`cluster-type` = "E"/"P"), which is what the
/// kernel orders `host_processor_info` by. Falls back to the perflevel sysctls,
/// assuming efficiency cores come first as they do on every M-series chip, and
/// to "no efficiency cores" on Intel.
fn efficiency_core_map() -> Vec<bool> {
    if let Some(map) = device_tree_core_map() {
        return map;
    }
    let e = sysctl_i32("hw.perflevel1.logicalcpu").unwrap_or(0).max(0) as usize;
    let p = sysctl_i32("hw.perflevel0.logicalcpu").unwrap_or(0).max(0) as usize;
    if e == 0 {
        return Vec::new();
    }
    (0..e + p).map(|i| i < e).collect()
}

fn device_tree_core_map() -> Option<Vec<bool>> {
    let mut port = 0;
    if unsafe { IOMasterPort(0, &mut port) } != 0 {
        return None;
    }
    let path = CString::new("IODeviceTree:/cpus").ok()?;
    let cpus = unsafe { IORegistryEntryFromPath(port, path.as_ptr() as *mut _) };
    if cpus == 0 {
        return None;
    }
    let mut iter = 0;
    let kr =
        unsafe { IORegistryEntryGetChildIterator(cpus, kIODeviceTreePlane as *mut _, &mut iter) };
    unsafe { IOObjectRelease(cpus) };
    if kr != 0 {
        return None;
    }
    let mut cores: Vec<(usize, bool)> = Vec::new();
    loop {
        let entry = unsafe { IOIteratorNext(iter) };
        if entry == 0 {
            break;
        }
        let cluster = registry_property(entry, "cluster-type").and_then(|v| cf_data_bytes(&v));
        let id = registry_property(entry, "logical-cpu-id").and_then(|v| cf_data_u32(&v));
        unsafe { IOObjectRelease(entry) };
        if let (Some(cluster), Some(id)) = (cluster, id) {
            cores.push((id as usize, cluster.first() == Some(&b'E')));
        }
    }
    unsafe { IOObjectRelease(iter) };
    if cores.is_empty() {
        return None;
    }
    let len = cores.iter().map(|(i, _)| i + 1).max().unwrap_or(0);
    let mut map = vec![false; len];
    for (i, e) in cores {
        map[i] = e;
    }
    Some(map)
}

// ---------------------------------------------------------------- GPU

/// Highest utilisation across all accelerators.
///
/// Apple Silicon and most discrete GPUs publish `Device Utilization %` in
/// `PerformanceStatistics`; some older AMD drivers use `GPU Activity(%)`.
fn sample_gpu() -> Option<GpuStats> {
    let mut port = 0;
    if unsafe { IOMasterPort(0, &mut port) } != 0 {
        return None;
    }
    let name = CString::new("IOAccelerator").ok()?;
    let mut iter = 0;
    let matching = unsafe { IOServiceMatching(name.as_ptr()) };
    if unsafe { IOServiceGetMatchingServices(port, matching, &mut iter) } != 0 {
        return None;
    }
    let mut best: Option<GpuStats> = None;
    loop {
        let entry = unsafe { IOIteratorNext(iter) };
        if entry == 0 {
            break;
        }
        let stats = registry_property(entry, "PerformanceStatistics")
            .and_then(|v| v.downcast::<CFDictionary>())
            .and_then(|d| gpu_stats_from(&d));
        unsafe { IOObjectRelease(entry) };
        if let Some(stats) = stats {
            if best.map_or(true, |b| stats.usage > b.usage) {
                best = Some(stats);
            }
        }
    }
    unsafe { IOObjectRelease(iter) };
    best
}

fn gpu_stats_from(dict: &CFDictionary) -> Option<GpuStats> {
    let usage = ["Device Utilization %", "GPU Activity(%)"]
        .iter()
        .find_map(|k| dict_number(dict, k))?;
    let memory_used = ["In use system memory", "vramUsedBytes"]
        .iter()
        .find_map(|k| dict_number(dict, k))
        .unwrap_or(0.0);
    Some(GpuStats {
        usage: (usage as f32).clamp(0.0, 100.0),
        memory_used: memory_used.max(0.0) as u64,
    })
}

// ---------------------------------------------------------------- memory

fn read_vm_stats(host: libc::mach_port_t) -> Option<libc::vm_statistics64> {
    let mut vm: libc::vm_statistics64 = unsafe { mem::zeroed() };
    let mut count = libc::HOST_VM_INFO64_COUNT;
    let kr = unsafe {
        libc::host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            &mut vm as *mut _ as libc::host_info64_t,
            &mut count,
        )
    };
    (kr == libc::KERN_SUCCESS).then_some(vm)
}

fn page_size() -> u64 {
    match unsafe { libc::sysconf(libc::_SC_PAGESIZE) } {
        n if n > 0 => n as u64,
        _ => 16384,
    }
}

fn swap_used() -> Option<u64> {
    let mut swap: libc::xsw_usage = unsafe { mem::zeroed() };
    sysctl_raw("vm.swapusage", &mut swap).then_some(swap.xsu_used)
}

// ---------------------------------------------------------------- network

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceCounters {
    pub name: String,
    pub rx: u64,
    pub tx: u64,
}

/// Physical and hardware-backed interfaces only.
///
/// Tunnels (`utun`, `ipsec`) carry traffic that also crosses the physical
/// link, so counting both would double VPN traffic; `awdl`/`llw` are AirDrop
/// peer links, `bridge`/`ap` mirror member interfaces, `anpi` is the internal
/// USB network to the SoC.
pub fn is_counted_interface(name: &str) -> bool {
    const SKIP: [&str; 10] = [
        "lo", "utun", "ipsec", "awdl", "llw", "bridge", "ap", "gif", "stf", "anpi",
    ];
    !SKIP.iter().any(|p| name.starts_with(p))
}

/// Bytes moved between two snapshots, plus the busiest interface.
///
/// Interfaces are matched by name. One that appeared, vanished or had its
/// counters reset contributes nothing instead of a huge spurious spike.
pub fn network_delta(
    prev: &[InterfaceCounters],
    now: &[InterfaceCounters],
) -> (u64, u64, Option<String>) {
    let mut down = 0;
    let mut up = 0;
    let mut busiest: Option<(&str, u64)> = None;
    for n in now {
        let Some(p) = prev.iter().find(|p| p.name == n.name) else {
            continue;
        };
        if n.rx < p.rx || n.tx < p.tx {
            continue;
        }
        let (d, u) = (n.rx - p.rx, n.tx - p.tx);
        down += d;
        up += u;
        if d + u > 0 && busiest.map_or(true, |(_, b)| d + u > b) {
            busiest = Some((&n.name, d + u));
        }
    }
    (down, up, busiest.map(|(n, _)| n.to_string()))
}

/// 64-bit per-interface byte counters via the `net.link.generic` MIB.
///
/// Neither `getifaddrs` nor `NET_RT_IFLIST2` is usable here: both truncate the
/// byte counters to 32 bits on current macOS (confirmed on 27.0, where an
/// interface at 8.28 GB reported 3.98 GB), so a long download would wrap every
/// 4 GiB. `IFMIB_IFDATA` returns the full `if_data64`.
fn read_interface_counters() -> Option<Vec<InterfaceCounters>> {
    let mut count: u32 = 0;
    let mut mib = [
        libc::CTL_NET,
        libc::PF_LINK,
        libc::NETLINK_GENERIC,
        libc::IFMIB_SYSTEM,
        libc::IFMIB_IFCOUNT,
    ];
    let mut len = mem::size_of::<u32>();
    let ok = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            &mut count as *mut u32 as *mut c_void,
            &mut len,
            ptr::null_mut(),
            0,
        )
    } == 0;
    if !ok {
        return None;
    }

    let mut out = Vec::new();
    // Interface rows are 1-based.
    for row in 1..=count as i32 {
        let mut mib = [
            libc::CTL_NET,
            libc::PF_LINK,
            libc::NETLINK_GENERIC,
            libc::IFMIB_IFDATA,
            row,
            libc::IFDATA_GENERAL,
        ];
        let mut data: libc::ifmibdata = unsafe { mem::zeroed() };
        let mut len = mem::size_of::<libc::ifmibdata>();
        let ok = unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                mib.len() as u32,
                &mut data as *mut _ as *mut c_void,
                &mut len,
                ptr::null_mut(),
                0,
            )
        } == 0;
        if !ok {
            // Rows can disappear between the count and the read.
            continue;
        }
        if data.ifmd_flags & libc::IFF_UP as u32 == 0 {
            continue;
        }
        let name = unsafe { CStr::from_ptr(data.ifmd_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        let stats = data.ifmd_data;
        out.push(InterfaceCounters {
            name,
            rx: stats.ifi_ibytes,
            tx: stats.ifi_obytes,
        });
    }
    Some(out)
}

// ---------------------------------------------------------------- helpers

fn sysctl_raw<T>(name: &str, out: &mut T) -> bool {
    let Ok(name) = CString::new(name) else {
        return false;
    };
    let mut len = mem::size_of::<T>();
    unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            out as *mut T as *mut c_void,
            &mut len,
            ptr::null_mut(),
            0,
        ) == 0
            && len == mem::size_of::<T>()
    }
}

fn sysctl_u64(name: &str) -> Option<u64> {
    let mut v = 0u64;
    sysctl_raw(name, &mut v).then_some(v)
}

fn sysctl_i32(name: &str) -> Option<i32> {
    let mut v = 0i32;
    sysctl_raw(name, &mut v).then_some(v)
}

fn registry_property(entry: u32, key: &str) -> Option<CFType> {
    let key = CFString::new(key);
    let value = unsafe {
        IORegistryEntryCreateCFProperty(entry, key.as_concrete_TypeRef(), kCFAllocatorDefault, 0)
    };
    (!value.is_null()).then(|| unsafe { CFType::wrap_under_create_rule(value) })
}

fn dict_number(dict: &CFDictionary, key: &str) -> Option<f64> {
    let key = CFString::new(key);
    let value = dict.find(key.as_concrete_TypeRef() as *const c_void)?;
    let value = unsafe { CFType::wrap_under_get_rule(*value as _) };
    value.downcast::<CFNumber>()?.to_f64()
}

fn cf_data_bytes(value: &CFType) -> Option<Vec<u8>> {
    value
        .downcast::<core_foundation::data::CFData>()
        .map(|d| d.bytes().to_vec())
}

fn cf_data_u32(value: &CFType) -> Option<u32> {
    let bytes = cf_data_bytes(value)?;
    let arr: [u8; 4] = bytes.get(..4)?.try_into().ok()?;
    Some(u32::from_le_bytes(arr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_usage_from_tick_deltas() {
        let prev = [[100, 50, 800, 50], [0, 0, 0, 0]];
        // Core 0: busy 60+30+10 of 200 → 50%. Core 1: busy 100 of 100.
        let now = [[160, 80, 900, 60], [50, 50, 0, 0]];
        let (all, cores) = cpu_usage(&prev, &now);
        assert!((cores[0] - 50.0).abs() < 1e-3);
        assert!((cores[1] - 100.0).abs() < 1e-3);
        assert!((all - 200.0 / 300.0 * 100.0).abs() < 1e-3);
    }

    #[test]
    fn cpu_usage_survives_counter_wrap_and_first_sample() {
        let prev = [[u32::MAX - 9, 0, 0, 0]];
        let now = [[10, 0, 20, 0]];
        let (all, _) = cpu_usage(&prev, &now);
        // 20 busy ticks across the wrap, 20 idle.
        assert!((all - 50.0).abs() < 1e-3);
        // No previous sample: zeros, not a panic or a bogus 100%.
        let (all, cores) = cpu_usage(&[], &now);
        assert_eq!(all, 0.0);
        assert_eq!(cores, vec![0.0]);
    }

    #[test]
    fn network_delta_ignores_resets_and_new_interfaces() {
        let c = |name: &str, rx, tx| InterfaceCounters {
            name: name.into(),
            rx,
            tx,
        };
        let prev = [c("en0", 1_000, 500), c("en1", 5_000, 5_000)];
        let now = [
            c("en0", 4_000, 700),
            c("en1", 10, 10),
            c("en7", 9_999, 9_999),
        ];
        let (down, up, busiest) = network_delta(&prev, &now);
        assert_eq!((down, up), (3_000, 200));
        assert_eq!(busiest.as_deref(), Some("en0"));
    }

    #[test]
    fn tunnels_and_peer_links_are_not_counted() {
        for name in [
            "lo0", "utun3", "awdl0", "llw0", "bridge0", "anpi1", "ipsec0",
        ] {
            assert!(!is_counted_interface(name), "{name}");
        }
        for name in ["en0", "en5", "pdp_ip0"] {
            assert!(is_counted_interface(name), "{name}");
        }
    }

    #[test]
    fn memory_pressure_levels() {
        assert_eq!(MemoryPressure::from_level(1), MemoryPressure::Normal);
        assert_eq!(MemoryPressure::from_level(2), MemoryPressure::Warning);
        assert_eq!(MemoryPressure::from_level(4), MemoryPressure::Critical);
        assert_eq!(MemoryPressure::from_level(0), MemoryPressure::Normal);
    }

    /// Reads the real machine. Asserts only invariants, since the values
    /// themselves depend on what is running.
    #[test]
    fn live_sample_is_sane() {
        let mut s = SystemSampler::new();
        let _ = s.sample();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let stats = s.sample();
        assert!(!stats.cpu.cores.is_empty());
        assert!((0.0..=100.0).contains(&stats.cpu.usage));
        assert!(stats.memory.total > 0);
        assert!(stats.memory.used <= stats.memory.total);
        assert!(stats.network.down_rate >= 0.0);
        if let Some(gpu) = stats.gpu {
            assert!((0.0..=100.0).contains(&gpu.usage));
        }
    }
}
