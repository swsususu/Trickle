<script setup lang="ts">
import type { BatteryHealthSnapshot } from '@/bindings'
import { commands } from '@/bindings'
import { Battery, TrendingDown } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const snapshots = ref<BatteryHealthSnapshot[]>([])
const isLoading = ref(true)

async function load() {
  const result = await commands.getBatteryHealthHistory()
  if (result.status === 'ok') {
    snapshots.value = result.data
  }
  else {
    console.error('[battery health]', result.error)
  }
  isLoading.value = false
}

onMounted(load)

/** Health percentage per recorded day. */
const chartData = computed(() =>
  snapshots.value
    .filter(s => s.designCapacity > 0)
    .map(s => ({
      day: s.day.slice(5), // MM-DD; the year is redundant on a dense axis
      [t('health.series')]: Number(((s.maxCapacity / s.designCapacity) * 100).toFixed(2)),
    })),
)

/** Change between the first and last recorded day. */
const delta = computed(() => {
  const data = chartData.value
  if (data.length < 2) {
    return null
  }
  const key = t('health.series')
  const first = data[0][key] as number
  const last = data[data.length - 1][key] as number
  return {
    value: (last - first).toFixed(2),
    days: data.length,
    declining: last < first,
  }
})
</script>

<template>
  <Card class="w-full">
    <CardHeader class="pb-0">
      <div class="flex items-center justify-between">
        <CardTitle class="flex items-center gap-2">
          {{ $t('health.title') }}
        </CardTitle>
        <div v-if="delta" class="flex items-center gap-1.5 text-xs font-mono text-muted-foreground">
          <TrendingDown v-if="delta.declining" class="size-3.5" />
          <span>{{ delta.value }}% / {{ $t('health.over_days', { days: delta.days }) }}</span>
        </div>
      </div>
    </CardHeader>
    <CardContent class="pt-4">
      <Skeleton v-if="isLoading" class="w-full h-[200px]" />

      <!-- One snapshot is written per day, so a new install has nothing to
           plot until tomorrow. Say so instead of showing an empty chart. -->
      <div
        v-else-if="chartData.length < 2"
        class="h-[200px] flex flex-col items-center justify-center gap-2 text-muted-foreground"
      >
        <Battery class="size-8 opacity-40" />
        <p class="text-sm">
          {{ $t('health.collecting') }}
        </p>
        <p class="text-xs opacity-70">
          {{ $t('health.collecting_desc') }}
        </p>
      </div>

      <LineChart
        v-else
        class="w-full h-[200px] font-bold"
        index="day"
        :data="chartData"
        :categories="[$t('health.series')]"
        :y-formatter="(value) => `${value}%`"
        :colors="['#2563eb']"
      />
    </CardContent>
  </Card>
</template>
