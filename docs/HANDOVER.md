# Trickle 项目交接说明

本文档记录 Trickle 从 powerflow 接手维护的决策依据、已完成工作与后续待办。
生成时间：2026-09-16。

## 一、项目背景

Trickle 基于 [lzt1008/powerflow](https://github.com/lzt1008/powerflow) 二次开发，MIT 许可。

上游停更证据（已核实）：

- 最后一次提交 2025-03-29（`2be325f`），距今约 18 个月
- 作者 `lzt1008` 近 30 条公开事件为空，30 个仓库中最近仅 2026-03 动过 `todo-cli`
- 22 个 open issue、5 个 open PR 全部零回应
- 关联的 `lzt1008/homebrew-powerflow` 同样停在 2025-03-29
- 仓库有 362 star、37 fork，未 archive

结论：不是放弃某个分支，而是放弃了整个项目。

## 二、macOS 27 崩溃根因（已在本机验证）

环境：macOS 27.0（build 26A428）、arm64。

实测 `ioreg -rn AppleSmartBattery` 结果：

| 键 | macOS 27 状态 |
|---|---|
| `AppleRawCurrentCapacity` | 缺失 |
| `AppleRawMaxCapacity` | 缺失 |
| `AbsoluteCapacity` | 缺失 |
| `DesignCapacity` | 缺失（移入 `BatteryData`） |
| `Temperature` | 缺失 |
| `CurrentCapacity` / `MaxCapacity` | 存在，但值为百分比 0-100 而非 mAh |
| `BatteryData`（嵌套 dict） | 存在：DesignCapacity 6249 / FullChargeCapacity 6166 / RemainingCapacity 6166 / NominalChargeCapacity 6318 |
| `TimeRemaining` | 65535（哨兵值，非真实分钟数） |

四个独立故障：

1. **启动崩溃** — `get_mac_ioreg().unwrap()` 因上述键缺失而失败。原版在本机实测直接 SIGSEGV（退出码 139），连 panic 信息都没有，因为 `mem::transmute` 在字段缺失时产生了未定义行为。
2. **图表冻结** — 旧版本可能持久化了 `statusBarItem:"none"`，而 `StatusBarItem` 用 serde 默认枚举实现，遇未知变体直接 panic；该 panic 发生在 power-tick 任务内部，静默杀死采样循环。另外历史遗留的 `updateInterval:86400000`（24 小时）会让定时器实际停摆。
3. **剩余时间错误** — SMC `B0TE`/`B0TF` 在 Apple Silicon 上更新极慢；macOS 27 上 SMC `CHCC` 可能在实际放电时仍报 charging=true，导致选中 `B0TF=65535` 哨兵，显示「1092 小时」。
4. **电池健康无数据** — 真实 mAh 值移入 `BatteryData`，原代码读顶层键得 0，健康比值无法计算。

## 三、为什么不直接合并 PR #22

[PR #22](https://github.com/lzt1008/powerflow/pull/22)（作者 @cnveteran）的修复质量确实不错，但它把两类改动混在一起，共 77 文件 / +2269 / -6322：

**值得采纳的修复**：serde 默认值与 `BatteryData` 解析、哨兵值过滤、去除 unsound 的 `mem::transmute`、补上漏掉的 `IOObjectRelease`（原代码每 2 秒泄漏一个 IOKit 对象）、`StatusBarItem` 未知变体降级、`SMCConnection::new().unwrap()` 改为可降级、定时器区间钳制、以及一个真 bug：后端读 pinia 键 `showCharging` 而前端存的是 `statusBarShowCharging`，导致「充电时显示功率」偏好一直失效。附 6 个单测。

**不应采纳的品牌化改动**：包名改 `mac-power-assistant`、productName 改「Mac电源助手」、identifier 改 `com.cnveteran.*`、版本从 0.2.2 回退到 0.1.0、全套图标替换为 Trae 生成图、LICENSE 添加自己的版权行、默认语言改 zh-CN、顺带重做 UI。

这解释了上游为什么不看：它读起来不像 bug fix，像 fork 后另起项目。

其余 open PR 状态：#21、#19 与 #22 在同一批文件上重叠（#19 也独立移除了 transmute），#11（iOS 设备指标）与 #16（繁体中文）是干净增量，可后续单独采纳。5 个 PR 均已拉为本地分支 `pr-11` / `pr-16` / `pr-19` / `pr-21` / `pr-22`。

## 四、命名决策

最终定名 **Trickle**，取自 trickle charge（涓流充电），术语根基真实、日常词好传播、意象贴合。

已验证占用情况：`sjmulder/trickle` 是 600 baud 管道小工具，`trickleapp/trickle-public` 是 AI 工作空间产品（不同领域），macOS menu bar 应用无同名。

已排除的候选（均已被占用，逐个查实）：

- Voltra — 778 star 的 React Native 库 + App Store「Voltra AI」+ 健身品牌
- Wattly — Tesla 伴侣应用 + Powerwall 服务 + 加州能耗合规软件
- Joulz — 荷兰能源公司；Amperia — Hexagon 商标
- PowerLens — `progresshans/powerlens` 已存在
- Power Monitor — SAP 官方同名 macOS 应用（251 star），另有至少 5 个同名仓库

配置决策：

- identifier：`com.swsususu.trickle`（发布后不可再改，否则用户配置与历史数据库会丢）
- 默认语言：保持 `en`，另补完整 zh-CN 翻译
- git 历史：保留上游完整历史（141 提交、4 个 tag），便于 blame 与追溯

## 五、跨平台与功能扩展结论

**不建议做 Windows / Linux 适配。** 项目核心价值全部依赖 macOS 私有 API：

- `crates/tpower/src/ffi/smc.rs`（427 行）走 Apple SMC 内核接口，读四字符 key（`PPBR`/`PDTR`/`PSTR`/`PHPC`/`PDBR`/`CHCC`/`B0TE`/`B0TF`/`TB0T`）
- `provider/mod.rs` 读 `AppleSmartBattery` IORegistry 属性
- `ffi/mod.rs`（156 行）link `MobileDevice.framework` 私有框架，20 余个符号
- `src-tauri/src/util.rs`（383 行）直接操作 `NSWindow`/`NSWindowButton`/自定义 `NSToolbar` delegate

Windows 只能拿到 `Win32_Battery` / `GetSystemPowerStatus`，Linux 只有 `/sys/class/power_supply/`，都无法提供屏幕功耗、散热管功耗、适配器效率损耗这类细分遥测。跨平台等于重写三套采集后端，产出是功能残缺的另一个应用。

**网络监控建议限定在功耗相关范围。** [exelban/stats](https://github.com/exelban/stats) 有 41842 star、每日活跃维护、Swift 原生，已覆盖 CPU/GPU/内存/磁盘/网络/传感器/电池/风扇。同类项目（`emgeorrk/pulse`、`filiphajduch420/Vitals`、`owieth/MacVitals`、`Alyetama/Pulse`、多个 PowerTop）极度饱和。

Trickle 相对 stats 的唯一优势是那 427 行 SMC 深度功耗解析与 iOS 设备功耗监控——stats 的电池模块只有基础信息，没有功率流拆解。扩成通用系统监控等于用 Tauri + Vue 重做一个功能更少的 stats，同时丢掉差异化。建议保持「电源专家工具」定位，网络监控只做与功耗相关的部分（如网络活动对功耗影响、外接设备 USB 供电）。

## 六、已完成工作

**1. 导入上游历史（已提交 `e5e300c`）**

```
git remote add upstream https://github.com/lzt1008/powerflow.git
git merge upstream/main --allow-unrelated-histories
```

解决两处 add/add 冲突：

- `LICENSE` — 保留上游 `Copyright (c) The Powerflow Team (original work)`，追加 `Copyright (c) 2026 swsususu (modifications and enhancements)`。MIT 要求原版权声明不可替换。
- `README.md` — 重写为 Trickle 说明，开头注明基于 lzt1008/powerflow 二次开发并附链接。

**2. 挑拣 PR #22 核心修复（已提交 `548f29c`）**

10 个修改文件：

| 文件 | 内容 |
|---|---|
| `crates/tpower/src/de.rs` | `BatteryData` struct、serde 默认值、`repr` 到 `out` 的 From 实现 |
| `crates/tpower/src/provider/mod.rs` | `real_capacity_from()`、`time_remain_from()`、`is_charging_local()`、`absolute_battery_level()`、补 `IOObjectRelease`、去除 transmute、6 个单测 |
| `crates/tpower/src/provider/remote.rs` | 去除 `mem::transmute` |
| `crates/tpower/src/ffi/mod.rs` | 新增 `AMDeviceRelease` 声明、修正若干指针可变性 |
| `crates/tpower/src/ffi/smc.rs` | 小幅整理 |
| `crates/tpower/src/ffi/wrapper.rs` | `start_service` 改返回 Result、session/reference 生命周期管理 |
| `src-tauri/src/device.rs` | 按 interface 跟踪连接、USB 优先于 WiFi、去除 unwrap |
| `src-tauri/src/event.rs` | `StatusBarItem` 反序列化未知值降级为 System |
| `src-tauri/src/local.rs` | panic-free ioreg、区间钳制、AppleSMC 不可用时降级、修正 pinia 键名 |
| `src/stores/preference.ts` | 清洗历史遗留的非法 `statusBarItem` / `updateInterval` |

注意：`device.rs` 与 `ffi/wrapper.rs` 强耦合，必须一起取——PR22 改了 `Device::new` 签名（增加 `owns_reference`）和 `start_service` 返回类型。附带收益是修掉 `AMDeviceRelease` 泄漏和 USB+WiFi 重复上报。

已剔除：改名、图标替换、版本回退、UI 重做、LICENSE 篡改、默认语言变更。

**验证结果**：

- `cargo check -p tpower` 通过
- `cargo check -p trickle` 通过（20 个 warning 均为上游遗留的 `extern` ABI 弃用提示，与本次改动无关）
- `cargo test -p tpower` 6 个单测全过
- 实机探针（macOS 27.0 / arm64）：原版 SIGSEGV，修复后正常解析，`BatteryData` 读出 6166/6249、电池健康 98.7%、`time_remaining=65535` 哨兵被正确识别为未知而非 1092 小时

**3. 改名为 Trickle（已提交 `ff753af`）**

| 文件 | 改动 |
|---|---|
| `src-tauri/Cargo.toml` | package `trickle`、lib `trickle_lib` |
| `src-tauri/src/main.rs` | `trickle_lib::run()` |
| `src-tauri/src/menu.rs` | 应用菜单标题 `Trickle` |
| `src-tauri/tauri.conf.json` | productName `Trickle`、identifier `com.swsususu.trickle` |
| `bump.config.ts` | `cargo update trickle tpower` |
| `package.json` | name 由脚手架默认 `tauri-app` 改为 `trickle` |
| `src/Settings.vue` | commit 链接指向 `swsususu/Trickle` |

`src/components/PowerFlow.vue` 是组件名，未改。文档与 README 中对 `lzt1008/powerflow` 的引用属署名，保留。

打包产物 `Info.plist` 已核实：CFBundleName / CFBundleDisplayName = `Trickle`，CFBundleIdentifier = `com.swsususu.trickle`。

**4. 修正 zh-CN 短格式时间翻译（已提交 `a349508`）**

8 个 `short_*_future` 键全部误用「前」，导致未来时间点倒着显示。已改为「后」。
键集合与 `en.yaml` 完全对齐，两侧各 93 个键，无缺失项。

**5. 修复 release 构建失败（已提交 `802caa8`）**

`pnpm tauri build` 在 `sqlx-macros` 处报 `mis-aligned LINKEDIT string pool` 而中断。

根因是 rust-lang/rust#157750：Cargo 自 1.77 起 release profile 默认 `strip = "debuginfo"`；macOS 部署目标 ≥ 12.0 启用 chained fixups 后，rustc 剥离产生的 dylib 其 `LC_SYMTAB.stroff` 不满足 8 字节对齐，macOS 27 的 dyld 直接拒绝 dlopen。proc-macro 是 dylib，因此编译期就失败。

解法是在根 `Cargo.toml` 加 `[profile.release.build-override] strip = false`，只作用于构建脚本与过程宏，最终二进制仍被剥离。

**6. 完整构建验证（已执行）**

- `pnpm install` 成功
- `pnpm build`（含 `vue-tsc --noEmit`）通过，4437 模块，无类型错误
- `cargo build --release -p trickle` 通过
- `pnpm tauri build` 成功产出 `target/release/bundle/macos/Trickle.app`

无需 nightly，本机 stable rustc 1.96.1 即可（PR22 作者的 nightly 说法不成立）。

**dmg 步骤本地失败，非代码问题**：`create-dmg` 的 AppleScript 在设置 Finder 窗口外观时超时（`AppleEvent 已超时 -1712`），`.app` 在此之前已正常生成。CI 用 GitHub 托管 runner，一般不触发此问题；若发生可考虑仅打 `app` target 或加 Finder 自动化授权。

## 七、状态栏面板修复（已提交 `badc732`、`6d784c0`）

面板一度完全打不开，查出三个叠加的缺陷，都来自上游：

**左键无 `Click` 事件。** 加日志观察托盘事件流，只有 `Enter`/`Move`/`Leave`，`Click` 一个都没有。根因是 tray-icon 建图标时调用了 `NSStatusItem.setMenu`——macOS 上一旦设了 menu，AppKit 就自己处理 button 点击，`mouseDown:` 到不了 tray-icon 依赖的 `TrayTarget` subview。`menu_on_left_click(false)` 只改 tray-icon 内部的 ivar，动不了 AppKit 这层；build 后再调 `set_show_menu_on_left_click` 能让菜单不弹，但 `Click` 依然为零。解法是建图标时不挂 menu，右键按下时临时挂上。

**右键菜单不弹。** 仅 `set_menu` 不够，AppKit 对该次按下的处理方式已确定。需挂上后主动 `performClick`。该调用阻塞至菜单关闭，因此对应的 mouse-up 可能永不到达，卸载菜单必须放在 `performClick` 返回之后而非 mouse-up 分支。

**面板只有骨架。** `usePower` 把 `document.visibilityState === 'hidden'` 当作加载中，而 popover 窗口配置为 `visible: false`，可见性状态恒为 hidden。数据一直在推送，仅渲染被挡。

**面板被全屏应用压住。** `tauri.conf.json` 的 `alwaysOnTop` 对此无效——`to_popover()` 把 contentView 移入 `NSViewController`，实际显示的是 NSPopover 自己的窗口。需在 `show_popover()` 后取 `contentViewController.view.window`，设 `NSPopUpMenuWindowLevel`（101）与 `CanJoinAllSpaces | FullScreenAuxiliary`。

## 八、新增功能（已提交）

**电池健康趋势。** 新表 `battery_health_snapshots`，每日一行 upsert，常年开机一年 365 行。存原始 mAh 而非算好的百分比，便于将来改健康度公式而不失效历史。已实测：拿含 1 条充电历史的现有库升级，旧数据完整保留，新迁移叠加在已有 3 个之上。

**应用耗电排行。** 用 `top -stats power` 读取，与活动监视器同一指标，无需特权 helper（`powermetrics` 要 root，牵扯 `SMJobBless` 与签名，不值得）。按需触发而非轮询：`top` 需采样两次才有能耗差值，一次约 1.5 秒，挂定时器上耗电比省的多。

**适配器详情。** `AdapterDetails` 里 `Description`、`AdapterPowerTier`、`MaxVoltage`、`MaxCurrent` 一直未读。注意区分：`adapter_watts`/`adapter_voltage`/`adapter_amperage` 全是**额定值**（85W 充电器上恒为 85W/20V/4.25A），实际功率是 SMC 的 `adapter_power`。曾误加 `adapter_rated_watts` 字段读同一个 `Watts`，属重复，已删除。信息做成徽章的 tooltip，不额外占纵向空间。

## 九、待办

1. Intel Mac 支持（issue #18、#20）——优先级高于跨平台，SMC key 在 Intel 机型上部分不同，用户就在 macOS 生态内。
2. 采纳 PR #11（iOS 设备指标）与 PR #16（繁体中文），两者是干净增量。
3. `Info.plist` 的 `LSMinimumSystemVersion` 是 Tauri 默认的 10.13，与实际依赖（objc2-app-kit、NSPopover、IOKit 电源接口）不符，需实测确定真实下限后修正。
4. 发布方式：自签名分发 dmg 需 Apple Developer 账号（99 USD/年），否则用户需手动绕过 Gatekeeper（上游 issue #1 即此问题）。Homebrew tap 需自建 `homebrew-trickle` 仓库。
5. 上游遗留的 20 个 `extern` ABI 弃用 warning 可在后续清理中统一加 `extern "C"`。
6. 可考虑的后续功能：充电上限限制（需特权 helper 写 SMC `CHWA`/`BCLM`，是唯一能真正延长电池寿命的功能）、低电量/满电/高温通知、周期性快照对比。
7. 图标是脚本生成的（`scripts/generate_icon.py`），几何可复现但缺少设计质感——液滴轮廓是数学曲线而非调过的贝塞尔，闪电转角是硬的，整体无内阴影/高光层次。若要提升观感需设计师出 SVG 后覆盖 `app-icon.png` 并重跑 `pnpm tauri icon`。

## 十、环境记录

```
macOS 27.0 (26A428) / arm64
rustc 1.96.1 (stable-aarch64-apple-darwin) / LLVM 22.1.2
Apple clang 21.0.0 / ld-27037.1
node v25.2.1 / pnpm 10.0.0-rc.0
git user: swsususu <58540721+swsususu@users.noreply.github.com>
origin:   git@github.com:swsususu/Trickle.git
upstream: https://github.com/lzt1008/powerflow.git
```

`node_modules` 已安装。`gh` CLI 未安装，本次所有 GitHub 查询通过 REST API 完成。
