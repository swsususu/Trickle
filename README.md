# Trickle

English | [简体中文](README.zh-CN.md)

A macOS menu bar power monitor: live power flow, battery health, and charging status.

Trickle is a fork of [lzt1008/powerflow](https://github.com/lzt1008/powerflow). The
upstream project has had no commits since March 2025 and crashes on macOS 26/27;
Trickle continues maintenance and fixes those failures.

![Trickle](docs/images/screenshot.png)

> Pre-release. No signed build is distributed yet, so a locally built app needs
> to be opened past Gatekeeper (see [Installing](#installing)).

## Features

- **Live power flow** — adapter input, system load, battery charge/discharge,
  screen and heatpipe draw, and adapter conversion loss
- **Battery health** — design capacity, full charge capacity, cycle count, and
  remaining time estimate
- **Health trend** — daily capacity snapshots plotted over time, so decay is
  visible across months. History survives upgrades.
- **Energy usage by app** — the same per-process energy impact Activity Monitor
  reports. Measured on demand rather than polled, because sampling costs about a
  second and running it on a timer would use more energy than it saves.
- **Adapter details** — hover the wattage badge for the live draw against the
  adapter's rating, negotiated USB-C PD tier, and conversion loss. A charger
  delivering well below its rating is the usual answer to "why is this charging
  slowly".
- **History** — charging sessions recorded with power detail
- **iOS devices** — monitor paired iOS/iPadOS devices over USB or Wi-Fi

## Requirements

Verified on macOS 27.0 / Apple Silicon. Older versions are expected to work but
are untested; the bundle's declared minimum is inherited from Tauri's default
and is not a tested floor. Intel Macs run but some SMC sensors are unavailable
there (see [#18](https://github.com/lzt1008/powerflow/issues/18)).

## Resource usage

Measured on macOS 27.0 / M-series, sampling every 5 seconds, using CPU time
deltas and physical footprint rather than the `%cpu` and RSS columns, which
report lifetime averages and shared mappings respectively:

| | CPU | Memory |
|---|---|---|
| Menu bar only | ~1.7% | 270-290 MB |
| Main window open | ~19% | ~450 MB |

The window-open figure is the cost of the live chart and the animated readout;
turning off animations in Settings reduces it. Memory is dominated by WebKit:
the settings window is created on demand rather than at launch, which keeps one
fewer web view resident.

## Installing

Trickle is not notarised, so macOS refuses to open it on first launch either
way. Right-click the app and choose Open, or clear the attribute:

```bash
xattr -dr com.apple.quarantine /Applications/Trickle.app
```

Installing through Homebrew does not avoid this — it only makes upgrades easier:

```bash
brew install --cask swsususu/tap/trickle
```

Or download the DMG from
[Releases](https://github.com/swsususu/Trickle/releases/latest).

Migrating from powerflow: the two are separate applications, with different
bundle identifiers, so Trickle will not replace an existing powerflow install and
will not inherit its history. Remove the old app if you do not want two menu bar
icons.

## Building from source

```bash
pnpm install
pnpm tauri build
```

Building the DMG through `create-dmg` can time out on macOS 27 while AppleScript
styles the Finder window. The `.app` is produced before that step, so
`pnpm tauri build --bundles app` is a working alternative.

## What is fixed relative to upstream

macOS 27 moved and removed several `AppleSmartBattery` keys, which is the root of
most of these:

| Key | macOS 27 |
|---|---|
| `AppleRawCurrentCapacity` / `AppleRawMaxCapacity` | gone |
| `DesignCapacity` / `Temperature` / `AbsoluteCapacity` | gone from the top level |
| `CurrentCapacity` / `MaxCapacity` | present, but a 0-100 percentage, not mAh |
| `BatteryData` (nested dict) | holds the real mAh values |
| `TimeRemaining` | `65535` sentinel |

- Startup crash: the unchecked unwrap on missing keys, combined with
  `mem::transmute` over a struct with absent fields, produced a SIGSEGV with no
  panic message
- Battery health showing no data, from the same missing keys
- "1092 hours to full": the `TimeRemaining` sentinel was formatted as a duration
- Charging state stuck on while resting at 100%, because SMC `CHCC` keeps
  reporting `1.0` there; IOKit's `FullyCharged` is now trusted first
- A plugged-in machine described as running on battery
- Chart freezing: a stale `statusBarItem` value panicked inside the power-tick
  task and silently killed the sampling loop
- An IOKit object leaked once every two seconds
- Status bar panel not opening at all: an attached tray menu makes AppKit swallow
  `mouseDown:`, so no click event reached the app
- The panel rendering only skeletons, because an NSPopover window always reports
  `visibilityState: hidden`
- The panel being buried under fullscreen apps
- `showCharging` preference key mismatch, which made the setting do nothing
- Release builds failing at `sqlx-macros` with "mis-aligned LINKEDIT string
  pool" ([rust-lang/rust#157750](https://github.com/rust-lang/rust/issues/157750))

The core battery fixes were cherry-picked from
[powerflow#22](https://github.com/lzt1008/powerflow/pull/22) by @cnveteran, whose
diagnosis was correct; the rebranding in that PR was left out so the fixes apply
on their own.

## License

MIT. Original copyright The Powerflow Team; modifications copyright swsususu.
See [LICENSE](LICENSE).
