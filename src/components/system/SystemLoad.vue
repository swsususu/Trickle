<script setup lang="ts">
import { formatBytes, formatRate } from '@/composables/useSystem'
import { CircuitBoard, Cpu, MemoryStick, Network } from 'lucide-vue-next'

const system = useSystem()
const { preference } = usePreferenceAsync()

const data = computed(() => system.data)
const history = computed(() => system.history)

const cores = computed(() => {
  const all = data.value?.cpu.cores ?? []
  // Efficiency cores first, matching how the kernel orders them on Apple
  // Silicon and how Activity Monitor draws them.
  return [...all.filter(c => c.efficiency), ...all.filter(c => !c.efficiency)]
})
const efficiencyCount = computed(() => cores.value.filter(c => c.efficiency).length)
const performanceCount = computed(() => cores.value.length - efficiencyCount.value)

const memory = computed(() => data.value?.memory)
const memorySegments = computed(() => {
  const m = memory.value
  if (!m?.total)
    return []
  return [
    { key: 'app', value: m.app, class: 'bg-blue-500' },
    { key: 'wired', value: m.wired, class: 'bg-indigo-500' },
    { key: 'compressed', value: m.compressed, class: 'bg-violet-500' },
  ].map(s => ({ ...s, width: `${(s.value / m.total) * 100}%` }))
})

const pressureClass = {
  normal: 'text-green-600 bg-green-500/10 dark:text-green-400',
  warning: 'text-amber-600 bg-amber-500/10 dark:text-amber-400',
  critical: 'text-red-600 bg-red-500/10 dark:text-red-400',
}

const down = computed(() => formatRate(data.value?.network.downRate ?? 0, preference.networkUnit))
const up = computed(() => formatRate(data.value?.network.upRate ?? 0, preference.networkUnit))

const visibleCount = computed(() =>
  [preference.showCpu, preference.showGpu && data.value?.gpu, preference.showMemory, preference.showNetwork]
    .filter(Boolean)
    .length,
)
const gridClass = computed(() => ({
  1: 'grid-cols-1',
  2: 'grid-cols-2',
  3: 'grid-cols-3',
  4: 'grid-cols-4',
}[visibleCount.value] ?? 'grid-cols-4'))
</script>

<template>
  <div v-if="preference.systemMonitorEnabled && visibleCount" class="space-y-2">
    <h2 class="text-sm font-semibold text-muted-foreground">
      {{ $t('system.title') }}
    </h2>
    <div class="grid gap-4" :class="gridClass">
      <!-- CPU -->
      <Card v-if="preference.showCpu">
        <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle class="text-sm font-medium">
            CPU
          </CardTitle>
          <Cpu class="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div v-if="data" class="text-2xl font-bold font-mono">
            {{ data.cpu.usage.toFixed(0) }}<span class="text-sm text-muted-foreground ml-0.5">%</span>
          </div>
          <Skeleton v-else class="w-16 h-8" />
          <div class="flex gap-[2px] mt-3 h-6 items-end">
            <template v-for="(core, i) in cores" :key="i">
              <div v-if="i === efficiencyCount && efficiencyCount" class="w-1 shrink-0" />
              <div class="flex-1 h-full bg-muted rounded-sm overflow-hidden relative">
                <div
                  class="absolute inset-x-0 bottom-0 transition-[height] duration-500"
                  :class="core.efficiency ? 'bg-cyan-500' : 'bg-blue-500'"
                  :style="{ height: `${core.usage}%` }"
                />
              </div>
            </template>
          </div>
          <div v-if="efficiencyCount" class="flex gap-3 mt-2 text-[10px] text-muted-foreground">
            <span class="flex items-center gap-1"><i class="size-1.5 rounded-sm bg-cyan-500" />{{ $t('system.efficiency_cores', { n: efficiencyCount }) }}</span>
            <span class="flex items-center gap-1"><i class="size-1.5 rounded-sm bg-blue-500" />{{ $t('system.performance_cores', { n: performanceCount }) }}</span>
          </div>
        </CardContent>
      </Card>

      <!-- GPU: hidden entirely when no accelerator reports utilisation. -->
      <Card v-if="preference.showGpu && data?.gpu">
        <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle class="text-sm font-medium">
            GPU
          </CardTitle>
          <CircuitBoard class="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div class="text-2xl font-bold font-mono">
            {{ data.gpu.usage.toFixed(0) }}<span class="text-sm text-muted-foreground ml-0.5">%</span>
          </div>
          <Sparkline
            class="mt-2"
            :values="history.map(h => h.gpu)"
            :max="100"
            color="#8b5cf6"
          />
          <p v-if="data.gpu.memoryUsed" class="text-xs text-muted-foreground mt-1 font-mono">
            {{ $t('system.gpu_memory') }} {{ formatBytes(data.gpu.memoryUsed) }}
          </p>
        </CardContent>
      </Card>

      <!-- Memory -->
      <Card v-if="preference.showMemory">
        <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle class="text-sm font-medium">
            {{ $t('system.memory') }}
          </CardTitle>
          <span
            v-if="memory"
            class="text-[10px] font-semibold px-1.5 py-px rounded-full"
            :class="pressureClass[memory.pressure]"
          >
            {{ $t(`system.pressure_${memory.pressure}`) }}
          </span>
          <MemoryStick v-else class="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div v-if="memory" class="text-2xl font-bold font-mono">
            {{ (memory.used / 1024 ** 3).toFixed(1) }}<span class="text-sm text-muted-foreground ml-1">/ {{ formatBytes(memory.total, 0) }}</span>
          </div>
          <Skeleton v-else class="w-24 h-8" />
          <div class="flex h-1.5 mt-3 rounded-full bg-muted overflow-hidden">
            <CommonTooltip v-for="seg in memorySegments" :key="seg.key" as-child>
              <template #popper>
                {{ $t(`system.mem_${seg.key}`) }} <span class="font-mono ml-1">{{ formatBytes(seg.value) }}</span>
              </template>
              <div class="h-full transition-[width] duration-500" :class="seg.class" :style="{ width: seg.width }" />
            </CommonTooltip>
          </div>
          <div class="flex flex-wrap gap-x-3 mt-2 text-[10px] text-muted-foreground">
            <span class="flex items-center gap-1"><i class="size-1.5 rounded-sm bg-blue-500" />{{ $t('system.mem_app') }}</span>
            <span class="flex items-center gap-1"><i class="size-1.5 rounded-sm bg-indigo-500" />{{ $t('system.mem_wired') }}</span>
            <span class="flex items-center gap-1"><i class="size-1.5 rounded-sm bg-violet-500" />{{ $t('system.mem_compressed') }}</span>
            <span v-if="memory" class="font-mono">{{ $t('system.swap') }} {{ formatBytes(memory.swapUsed) }}</span>
          </div>
        </CardContent>
      </Card>

      <!-- Network -->
      <Card v-if="preference.showNetwork">
        <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle class="text-sm font-medium">
            {{ $t('system.network') }}
          </CardTitle>
          <span v-if="data?.network.interface" class="text-[11px] font-mono text-muted-foreground">
            {{ data.network.interface }}
          </span>
          <Network v-else class="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div class="flex gap-4 font-mono">
            <div>
              <span class="text-emerald-500 font-bold">↓</span>
              <span class="text-lg font-bold ml-1">{{ down[0] }}</span><span class="text-xs text-muted-foreground ml-0.5">{{ down[1] }}</span>
            </div>
            <div>
              <span class="text-amber-500 font-bold">↑</span>
              <span class="text-lg font-bold ml-1">{{ up[0] }}</span><span class="text-xs text-muted-foreground ml-0.5">{{ up[1] }}</span>
            </div>
          </div>
          <Sparkline
            class="mt-2"
            :values="history.map(h => h.down)"
            :second="history.map(h => h.up)"
            color="#10b981"
            second-color="#f59e0b"
          />
          <!-- One line even at 230px: the label moves into a tooltip. -->
          <CommonTooltip v-if="data" :content="$t('system.since_boot')" as-child>
            <p class="text-xs text-muted-foreground mt-1 font-mono whitespace-nowrap overflow-hidden text-ellipsis">
              Σ ↓{{ formatBytes(data.network.totalDown) }} ↑{{ formatBytes(data.network.totalUp) }}
            </p>
          </CommonTooltip>
        </CardContent>
      </Card>
    </div>
  </div>
</template>
