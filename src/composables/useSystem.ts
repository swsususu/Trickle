import type { SystemStats } from '@/bindings'
import { events } from '@/bindings'
import { reactive } from 'vue'
import { usePowerData } from './usePower'

/** Samples kept for sparklines and the load × power chart. */
const MAX_HISTORY = 60

export interface SystemHistoryPoint {
  time: number
  cpu: number
  gpu: number
  memory: number
  down: number
  up: number
  /** System power at the same tick, so load and watts share a time axis. */
  power: number
}

interface SystemState {
  data: SystemStats | null
  history: SystemHistoryPoint[]
}

const state: SystemState = reactive({
  data: null,
  history: [],
})

events.systemTickEvent.listen(({ payload: { data } }) => {
  state.data = data
  state.history.push({
    time: Date.now(),
    cpu: data.cpu.usage,
    gpu: data.gpu?.usage ?? 0,
    memory: memoryPercent(data),
    down: data.network.downRate,
    up: data.network.upRate,
    // The power tick for this sample is emitted right after this event, so
    // this is the previous reading, one interval old: close enough for a
    // trend line, and it avoids pairing events by timestamp.
    power: usePowerData().local.data.systemLoad ?? 0,
  })
  if (state.history.length > MAX_HISTORY)
    state.history.shift()
})

export function memoryPercent(data: SystemStats) {
  return data.memory.total ? (data.memory.used / data.memory.total) * 100 : 0
}

export function useSystem() {
  return state
}

/** `1.2 MB/s`, or `9.6 Mbps` when bits are preferred. */
export function formatRate(bytesPerSec: number, unit: 'bytes' | 'bits' = 'bytes'): [string, string] {
  if (unit === 'bits') {
    const bits = bytesPerSec * 8
    if (bits >= 1e9)
      return [(bits / 1e9).toFixed(1), 'Gbps']
    if (bits >= 1e6)
      return [(bits / 1e6).toFixed(1), 'Mbps']
    return [(bits / 1e3).toFixed(0), 'Kbps']
  }
  if (bytesPerSec >= 1024 ** 3)
    return [(bytesPerSec / 1024 ** 3).toFixed(1), 'GB/s']
  if (bytesPerSec >= 1024 ** 2)
    return [(bytesPerSec / 1024 ** 2).toFixed(1), 'MB/s']
  return [(bytesPerSec / 1024).toFixed(0), 'KB/s']
}

export function formatBytes(bytes: number, digits = 1) {
  if (bytes >= 1024 ** 4)
    return `${(bytes / 1024 ** 4).toFixed(digits)} TB`
  if (bytes >= 1024 ** 3)
    return `${(bytes / 1024 ** 3).toFixed(digits)} GB`
  if (bytes >= 1024 ** 2)
    return `${(bytes / 1024 ** 2).toFixed(0)} MB`
  return `${(bytes / 1024).toFixed(0)} KB`
}
