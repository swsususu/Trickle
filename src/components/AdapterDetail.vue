<script setup lang="ts">
import { Plug, Zap } from 'lucide-vue-next'

const power = usePower()

/** Rated wattage vs. what is actually being drawn right now. */
const draw = computed(() => {
  const rated = power.value.adapterRatedWatts ?? 0
  const actual = power.value.adapterWatts ?? 0
  if (!rated || !actual) {
    return null
  }
  return {
    rated,
    actual,
    // A charger negotiating below its rating is the usual reason for slow
    // charging, so surface the ratio rather than making users compare numbers.
    percent: Math.min(100, Math.round((actual / rated) * 100)),
    // Only call it limited when the gap is wide enough to matter; small
    // deviations are normal regulation.
    limited: actual < rated * 0.8,
  }
})
</script>

<template>
  <Card v-if="power.externalConnected">
    <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
      <CardTitle class="text-sm font-medium">
        {{ $t('adapter.title') }}
      </CardTitle>
      <Plug class="h-4 w-4 text-muted-foreground" />
    </CardHeader>
    <CardContent>
      <Skeleton v-if="power.isLoading" class="w-full h-20" />
      <template v-else>
        <div class="flex items-baseline gap-2">
          <span class="text-2xl font-bold">{{ power.adapterWatts }}W</span>
          <span v-if="draw" class="text-sm text-muted-foreground font-mono">
            / {{ draw.rated }}W
          </span>
        </div>

        <div class="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground font-mono">
          <span>{{ power.adapterVoltage }}V · {{ power.adapterAmperage }}A</span>
          <span v-if="power.adapterDescription">{{ power.adapterDescription }}</span>
          <span v-if="power.adapterPowerTier">
            {{ $t('adapter.tier') }} {{ power.adapterPowerTier }}
          </span>
          <span v-if="power.adapterIsWireless">{{ $t('adapter.wireless') }}</span>
        </div>

        <!-- Explain a charger that is delivering well below its rating. -->
        <div v-if="draw?.limited" class="mt-2 flex items-start gap-1.5 text-xs text-amber-600 dark:text-amber-500">
          <Zap class="size-3.5 shrink-0 mt-px" />
          <span>{{ $t('adapter.limited', { percent: draw.percent }) }}</span>
        </div>
      </template>
    </CardContent>
  </Card>
</template>
