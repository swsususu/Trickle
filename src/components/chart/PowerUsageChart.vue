<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import CustomChartTooltip from './CustomChartTooltip.vue'

const { t } = useI18n()
const power = usePower()

const categories = computed(() => {
  const base = ['System Power'] as (keyof StatisticData)[]
  if (!power.value.isRemote) {
    base.push('Screen Power', 'Heatpipe Power')
  }
  if (power.value.isCharging) {
    base.push('System In')
  }
  return base
})

const localeMap = computed(() => ({
  'System Power': t('flow.system_total'),
  'Screen Power': t('flow.screen_power'),
  'Heatpipe Power': t('flow.heatpipe_power'),
  'System In': t('flow.system_in'),
}))

// Read statistics from the raw store rather than through usePower(), which
// rebuilds its result object on every tick. Going through it made this map
// re-run and hand the chart a brand new array five times more often than the
// data actually changes, since statistics only grow once every
// LOCAL_UPDATE_INTERVAL ticks.
const powerData = usePowerData()
const tab = useTab()
const statistics = computed(() =>
  tab.value === 'local'
    ? powerData.local.statistics
    : powerData.remote[tab.value]?.statistics ?? [],
)

const localedData = computed(() => {
  return statistics.value.map((item) => {
    return Object.fromEntries(Object.entries(item).map(([key, value]) => [localeMap.value[key] || key, value]))
  })
})

const localedCategories = computed(() => categories.value.map(item => localeMap.value[item] || item))

// needs to force update chart when categories change, a bug of chart
const loading = ref(false)
watch([localeMap, () => power.value.isCharging], async () => {
  loading.value = true
  await nextTick()
  loading.value = false
})
</script>

<template>
  <Card class="w-full space-y-8 relative">
    <CardHeader class="pb-0">
      <CardTitle>
        {{ $t('power_usage') }}
      </CardTitle>
    </CardHeader>
    <CardContent>
      <Skeleton v-if="power.isLoading || loading" class="w-full h-[240px]" />
      <LineChart
        v-else
        class="w-full h-[240px] font-bold"
        index="time"
        :y-formatter="(value) => `${value}w`"
        :data="localedData"
        :categories="localedCategories"
        :custom-tooltip="CustomChartTooltip"
        :colors="['#2563eb', '#60a5fa', '#818cf8', '#0891b2']"
      />
    </CardContent>
  </Card>
</template>
