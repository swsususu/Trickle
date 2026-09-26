<script setup lang="ts">
import { commands } from '@/bindings'

const system = useSystem()
const { preference } = usePreferenceAsync()

const data = computed(() => system.data)

function heat(v: number) {
  return v > 80 ? 'bg-red-500' : v > 60 ? 'bg-amber-500' : 'bg-blue-500'
}

const pressureDot = {
  normal: 'bg-green-500',
  warning: 'bg-amber-500',
  critical: 'bg-red-500',
}

/** `1.2M` / `86K`: the cell is too narrow for units. */
function short(bytes: number) {
  const [v, unit] = formatRate(bytes, preference.networkUnit)
  return `${v}${unit[0]}`
}

const cells = computed(() => {
  const d = data.value
  if (!d)
    return []
  const out: { key: string, label: string, value: number, bar: string }[] = []
  if (preference.showCpu)
    out.push({ key: 'cpu', label: 'CPU', value: d.cpu.usage, bar: heat(d.cpu.usage) })
  if (preference.showGpu && d.gpu)
    out.push({ key: 'gpu', label: 'GPU', value: d.gpu.usage, bar: 'bg-violet-500' })
  if (preference.showMemory)
    out.push({ key: 'memory', label: 'memory', value: memoryPercent(d), bar: 'bg-indigo-500' })
  return out
})

const visible = computed(() =>
  preference.systemMonitorEnabled
  && preference.popoverSystemStats
  && !!data.value
  && (cells.value.length > 0 || preference.showNetwork),
)
const columns = computed(() => cells.value.length + (preference.showNetwork ? 1 : 0))
</script>

<template>
  <div v-if="visible && data" class="px-6 pb-4 cursor-pointer" @click="commands.openApp()">
    <div class="h-px bg-border -mx-6 mb-3" />
    <div class="grid gap-2" :style="{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }">
      <div v-for="c in cells" :key="c.key" class="rounded-lg bg-muted/60 px-2 py-1.5">
        <div class="flex items-center justify-between text-[10px] text-muted-foreground">
          <span>{{ c.key === 'memory' ? $t('system.memory') : c.label }}</span>
          <span v-if="c.key === 'memory'" class="size-1.5 rounded-full" :class="pressureDot[data.memory.pressure]" />
        </div>
        <div class="text-sm font-bold font-mono leading-5">
          {{ c.value.toFixed(0) }}<span class="text-[10px] text-muted-foreground">%</span>
        </div>
        <div class="h-[3px] rounded-full bg-border mt-1 overflow-hidden">
          <div class="h-full transition-[width] duration-500" :class="c.bar" :style="{ width: `${Math.min(100, c.value)}%` }" />
        </div>
      </div>
      <div v-if="preference.showNetwork" class="rounded-lg bg-muted/60 px-2 py-1.5">
        <div class="text-[10px] text-muted-foreground">
          {{ $t('system.network') }}
        </div>
        <div class="text-[11px] font-bold font-mono leading-[14px] mt-0.5">
          <div>↓ {{ short(data.network.downRate) }}</div>
          <div class="text-muted-foreground">
            ↑ {{ short(data.network.upRate) }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
