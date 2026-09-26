<script setup lang="ts">
import { formatRate } from '@/composables/useSystem'
/**
 * System power (area, left axis) against load (lines, right axis) on one
 * time axis. This is the reason system load lives in a power tool: it shows
 * which kind of work the watts are going to.
 */
const system = useSystem()
const { preference } = usePreferenceAsync()

const W = 960
const H = 180

type SeriesKey = 'cpu' | 'gpu' | 'memory' | 'network'
const series = reactive<Record<SeriesKey, boolean>>({
  cpu: true,
  gpu: true,
  memory: false,
  network: false,
})
const colors: Record<SeriesKey, string> = {
  cpu: '#3b82f6',
  gpu: '#8b5cf6',
  memory: '#6366f1',
  network: '#10b981',
}

const available = computed(() => ({
  cpu: preference.showCpu,
  gpu: preference.showGpu && !!system.data?.gpu,
  memory: preference.showMemory,
  network: preference.showNetwork,
}))

const history = computed(() => system.history)
const capacity = 60
const step = W / (capacity - 1)

/** Round the power ceiling up to a readable gridline value. */
const powerMax = computed(() => {
  const peak = Math.max(10, ...history.value.map(h => h.power))
  return Math.ceil(peak / 10) * 10
})

function path(values: number[], max: number) {
  if (values.length < 2)
    return ''
  const offset = (capacity - values.length) * step
  return values
    .map((v, i) => `${i ? 'L' : 'M'}${(offset + i * step).toFixed(1)},${(H - Math.min(1, v / max) * (H - 8)).toFixed(1)}`)
    .join('')
}

const powerLine = computed(() => path(history.value.map(h => h.power), powerMax.value))
const powerArea = computed(() => {
  if (!powerLine.value)
    return ''
  const startX = powerLine.value.slice(1).split(',')[0]
  return `${powerLine.value}L${W},${H}L${startX},${H}Z`
})

/**
 * Network has no natural ceiling, so it is plotted on a log scale mapped to
 * 0-100: 1 KB/s near the bottom, 100 MB/s at the top.
 */
function networkPercent(bytes: number) {
  if (bytes <= 1024)
    return 0
  return Math.min(100, (Math.log10(bytes / 1024) / 5) * 100)
}

const lines = computed(() => {
  const out: { key: SeriesKey, d: string }[] = []
  for (const key of Object.keys(series) as SeriesKey[]) {
    if (!series[key] || !available.value[key])
      continue
    const values = history.value.map(h =>
      key === 'network' ? networkPercent(h.down + h.up) : h[key],
    )
    out.push({ key, d: path(values, 100) })
  }
  return out
})

const hover = ref<number | null>(null)
const svg = useTemplateRef<SVGSVGElement>('svg')
function onMove(e: MouseEvent) {
  const rect = svg.value?.getBoundingClientRect()
  if (!rect || !history.value.length)
    return
  const x = ((e.clientX - rect.left) / rect.width) * W
  const offset = (capacity - history.value.length) * step
  const i = Math.round((x - offset) / step)
  hover.value = i >= 0 && i < history.value.length ? i : null
}
const hovered = computed(() => (hover.value == null ? null : history.value[hover.value]))
const hoverX = computed(() =>
  hover.value == null ? 0 : ((capacity - history.value.length + hover.value) * step) / W * 100,
)
</script>

<template>
  <Card v-if="preference.systemMonitorEnabled" class="w-full">
    <CardHeader class="pb-2">
      <div class="flex items-start justify-between gap-4">
        <div>
          <CardTitle>{{ $t('system.load_vs_power') }}</CardTitle>
          <p class="text-xs text-muted-foreground mt-1">
            {{ $t('system.load_vs_power_desc') }}
          </p>
        </div>
        <div class="flex gap-3 text-xs shrink-0">
          <label
            v-for="key in (Object.keys(series) as SeriesKey[])"
            v-show="available[key]"
            :key="key"
            class="flex items-center gap-1.5 cursor-pointer select-none"
          >
            <input
              v-model="series[key]"
              type="checkbox"
              class="rounded-sm size-3"
              :style="{ color: colors[key] }"
            >
            <span :style="{ color: colors[key] }" class="font-medium">{{ $t(`system.series_${key}`) }}</span>
          </label>
        </div>
      </div>
    </CardHeader>
    <CardContent>
      <Skeleton v-if="history.length < 2" class="w-full h-[180px]" />
      <div v-else class="relative" @mouseleave="hover = null">
        <svg
          ref="svg"
          class="w-full h-[180px] block"
          :viewBox="`0 0 ${W} ${H}`"
          preserveAspectRatio="none"
          @mousemove="onMove"
        >
          <line
            v-for="i in 3"
            :key="i"
            x1="0"
            :x2="W"
            :y1="(H * i) / 4"
            :y2="(H * i) / 4"
            stroke="hsl(var(--border))"
            stroke-dasharray="3 4"
            vector-effect="non-scaling-stroke"
          />
          <path :d="powerArea" fill="#3b82f6" opacity="0.1" />
          <path
            :d="powerLine"
            fill="none"
            stroke="#3b82f6"
            stroke-opacity="0.45"
            stroke-width="1"
            vector-effect="non-scaling-stroke"
          />
          <path
            v-for="l in lines"
            :key="l.key"
            :d="l.d"
            fill="none"
            :stroke="colors[l.key]"
            stroke-width="2"
            vector-effect="non-scaling-stroke"
          />
        </svg>
        <span class="absolute left-1 top-0 text-[10px] font-mono text-muted-foreground">{{ powerMax }}W</span>
        <span class="absolute left-1 bottom-0 text-[10px] font-mono text-muted-foreground">0</span>
        <span class="absolute right-1 top-0 text-[10px] font-mono text-muted-foreground">100%</span>

        <template v-if="hovered">
          <div class="absolute top-0 bottom-0 w-px bg-foreground/20 pointer-events-none" :style="{ left: `${hoverX}%` }" />
          <div
            class="absolute top-2 pointer-events-none rounded-md border bg-background/95 shadow-sm px-2 py-1.5 text-[11px] font-mono space-y-0.5"
            :class="hoverX > 70 ? '-translate-x-full -ml-2' : 'ml-2'"
            :style="{ left: `${hoverX}%` }"
          >
            <div class="text-muted-foreground">
              {{ new Date(hovered.time).toLocaleTimeString() }}
            </div>
            <div class="font-bold">
              {{ hovered.power.toFixed(1) }}W
            </div>
            <div v-if="available.cpu" :style="{ color: colors.cpu }">
              CPU {{ hovered.cpu.toFixed(0) }}%
            </div>
            <div v-if="available.gpu" :style="{ color: colors.gpu }">
              GPU {{ hovered.gpu.toFixed(0) }}%
            </div>
            <div v-if="available.memory" :style="{ color: colors.memory }">
              {{ $t('system.series_memory') }} {{ hovered.memory.toFixed(0) }}%
            </div>
            <div v-if="available.network" :style="{ color: colors.network }">
              ↓{{ formatRate(hovered.down, preference.networkUnit).join(' ') }} ↑{{ formatRate(hovered.up, preference.networkUnit).join(' ') }}
            </div>
          </div>
        </template>
      </div>
    </CardContent>
  </Card>
</template>
