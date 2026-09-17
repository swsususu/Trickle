<script setup lang="ts">
import { addSeconds, format } from 'date-fns'
import { ArrowUpDown } from 'lucide-vue-next'

const power = usePower()
const rawData = usePowerRaw()

const showRemainDuration = ref(true)
const buttonText = computed(() => {
  // IOKit reports 65535 / -1 while it is still estimating; timeRemain is
  // then zero and must not be rendered as "0h 0m".
  if (!power.value.timeRemainKnown) {
    return '--'
  }
  if (showRemainDuration.value) {
    const minutes = power.value.timeRemain.secs / 60
    const hours = Math.floor(minutes / 60)

    return `${hours}h ${minutes % 60}m`
  }
  return format(
    addSeconds(new Date(), power.value.timeRemain.secs),
    'HH:mm',
  )
})
</script>

<template>
  <div class="mr-10 flex gap-2 items-center">
    {{ power.isCharging ? $t('status.charging_power') : $t('status.system_power') }}
    <span
      v-if="power.isRemote"
      class="mr-1 size-2 rounded-full"
      :class="{
        'bg-blue-500 animate-pulse': !rawData.isLocal && !rawData.offline,
        'bg-neutral-500': !rawData.isLocal && rawData.offline,
      }"
    />
  </div>

  <Skeleton v-if="power.isLoading" class="w-24 h-6" />
  <!-- Show adapter specs whenever it is plugged in. While resting at 100% the
       battery is not charging and has no meaningful remaining time, so the
       adapter is the useful thing to report.

       The badge shows what the adapter advertises: AdapterDetails reports a
       fixed rating (85W / 20V / 4.25A on an 85W charger) no matter what is
       actually flowing. The live draw comes from the SMC, and is put in the
       tooltip along with the details that do not fit on the badge. -->
  <CommonTooltip
    v-else-if="power.isCharging || (power.externalConnected && !power.timeRemainKnown)"
    as-child
  >
    <div
      class="rounded-md
      bg-gradient-to-r from-blue-500 to-blue-600
      px-2 py-1 text-xs truncate font-mono cursor-default"
    >
      <span class="font-bold mr-1 text-background">{{ power.adapterWatts }}W</span>
      <span class="text-[10px] text-background/80">({{ power.adapterVoltage }}V,{{
        power.adapterAmperage }}A)</span>
    </div>
    <template #popper>
      <div class="flex flex-col gap-0.5 text-xs font-mono font-normal">
        <div class="flex justify-between gap-4">
          <span class="text-muted-foreground">{{ $t('adapter.drawing') }}</span>
          <span class="font-bold">{{ (power.adapterPower ?? 0).toFixed(1) }}W</span>
        </div>
        <div class="flex justify-between gap-4">
          <span class="text-muted-foreground">{{ $t('adapter.rated') }}</span>
          <span>{{ power.adapterWatts }}W</span>
        </div>
        <div v-if="power.adapterPowerTier" class="flex justify-between gap-4">
          <span class="text-muted-foreground">{{ $t('adapter.tier') }}</span>
          <span>{{ power.adapterPowerTier }}</span>
        </div>
        <div v-if="power.adapterDescription" class="flex justify-between gap-4">
          <span class="text-muted-foreground">{{ $t('adapter.type') }}</span>
          <span>{{ power.adapterIsWireless ? $t('adapter.wireless') : power.adapterDescription }}</span>
        </div>
        <div v-if="power.efficiencyLoss" class="flex justify-between gap-4">
          <span class="text-muted-foreground">{{ $t('adapter.loss') }}</span>
          <span>{{ power.efficiencyLoss.toFixed(2) }}W</span>
        </div>
      </div>
    </template>
  </CommonTooltip>
  <div
    v-else
    class="rounded-md dark:bg-blue-600 bg-blue-600 px-2 py-1 text-xs truncate font-mono w-20 text-background flex items-center justify-center
            cursor-pointer hover:bg-blue-600 transition-colors
            "
    @click.stop="showRemainDuration = !showRemainDuration"
  >
    <span class="font-bold mr-1">{{ buttonText }}</span>
    <ArrowUpDown
      class="size-3 text-background/80 transition-transform duration-300"
      :class="{ 'rotate-180': showRemainDuration }"
    />
  </div>
</template>
