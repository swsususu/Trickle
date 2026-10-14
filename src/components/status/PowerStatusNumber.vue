<script setup lang="ts">
import NumberFlow from '@number-flow/vue'

const power = usePower()
const { preference, isLoading } = usePreferenceAsync()

// Only animate while the web view is actually on screen.
//
// WebKit stops a hidden document's animation timeline (the status bar panel
// is hidden whenever it is closed, which is nearly always), so the Web
// Animations NumberFlow starts on every tick never finish. They pile up at
// about one per digit per tick, and each style recalc walks all of them:
// measured 22 live animations after 5 s, 122 after 80 s, with the panel's
// WebContent CPU climbing for as long as the app runs (issue #2). The plain
// readout below costs nothing, and NumberFlow re-mounts with the current value
// when the panel opens.
const visibility = useDocumentVisibility()
const animate = computed(() => preference.animationsEnabled && visibility.value === 'visible')
</script>

<template>
  <div>
    <Skeleton v-if="power.isLoading || isLoading" class="w-40 h-[50px] mt-2" />
    <NumberFlow
      v-else-if="animate"
      class="text-4xl font-bold"
      :format="{ maximumFractionDigits: 1, minimumFractionDigits: 1 }"
      :value="power.isCharging ? power.systemIn : power.systemLoad"
      suffix="w"
    />
    <div v-else class="text-4xl leading-[54px] font-bold">
      {{ (power.isCharging ? power.systemIn : power.systemLoad).toFixed(1) }}w
    </div>
  </div>
</template>
