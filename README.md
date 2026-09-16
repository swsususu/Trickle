# Trickle

macOS 菜单栏电源监控工具，实时查看功率流向、电池健康与充电状态。

Trickle 基于 [lzt1008/powerflow](https://github.com/lzt1008/powerflow) 二次开发。上游项目自 2025 年 3 月起停止维护，Trickle 接续维护并修复了 macOS 26/27 上的兼容性问题。

> 开发中，尚未发布正式版本。

## 功能

- **实时功率流** — 适配器输入、系统负载、电池充放电功率、屏幕与散热功耗、适配器效率损耗
- **电池健康** — 设计容量、满充容量、循环次数、剩余时间估算
- **历史趋势** — 功耗历史记录与图表
- **iOS 设备** — 通过 USB 或 Wi-Fi 监控已配对 iOS/iPadOS 设备的功耗

## 系统要求

macOS 13 或更高版本。Apple Silicon 与 Intel Mac 均可运行，但部分 SMC 传感器在 Intel 机型上不可用。

## 从源码构建

```bash
pnpm install
pnpm tauri build
```

## 与上游的差异

- 修复 macOS 27 上因 `AppleSmartBattery` 键位变更导致的启动崩溃
- 修复图表冻结、剩余时间显示异常、电池健康无数据
- 移除 unsound 的 `mem::transmute` 用法，修复 IOKit 对象泄漏
- 修复状态栏"充电时显示功率"偏好项失效

## 许可

MIT License。原始版权归 The Powerflow Team，修改部分版权归 swsususu。详见 [LICENSE](LICENSE)。
