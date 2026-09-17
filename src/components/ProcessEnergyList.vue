<script setup lang="ts">
import type { ProcessEnergy } from '@/bindings'
import { commands } from '@/bindings'
import { Activity, RefreshCw } from 'lucide-vue-next'

const processes = ref<ProcessEnergy[]>([])
const isLoading = ref(false)
const hasLoaded = ref(false)

/**
 * Reading this costs ~1.5s, because `top` needs two samples before it can
 * report an energy delta. It is therefore explicitly user-triggered rather
 * than polled: refreshing it on a timer would burn more energy than the
 * information saves.
 */
async function load() {
  if (isLoading.value) {
    return
  }
  isLoading.value = true
  const result = await commands.getProcessEnergy()
  processes.value = result
  isLoading.value = false
  hasLoaded.value = true
}

const maxImpact = computed(() =>
  processes.value.reduce((max, p) => Math.max(max, p.impact), 0),
)
</script>

<template>
  <Card class="w-full">
    <CardHeader class="pb-2">
      <div class="flex items-center justify-between">
        <CardTitle class="text-sm font-medium">
          {{ $t('processes.title') }}
        </CardTitle>
        <div
          class="rounded-md p-1.5 hover:bg-muted transition-colors cursor-pointer"
          @click="load"
        >
          <CommonTooltip :content="$t('processes.refresh')" as-child>
            <RefreshCw
              class="size-4 text-muted-foreground"
              :class="{ 'animate-spin': isLoading }"
            />
          </CommonTooltip>
        </div>
      </div>
    </CardHeader>
    <CardContent>
      <!-- Not fetched yet: the measurement is slow, so it waits to be asked. -->
      <div
        v-if="!hasLoaded && !isLoading"
        class="py-6 flex flex-col items-center justify-center gap-2 text-muted-foreground cursor-pointer"
        @click="load"
      >
        <Activity class="size-7 opacity-40" />
        <p class="text-sm">
          {{ $t('processes.measure') }}
        </p>
        <p class="text-xs opacity-70">
          {{ $t('processes.measure_desc') }}
        </p>
      </div>

      <div v-else-if="isLoading" class="space-y-2 py-1">
        <Skeleton v-for="i in 5" :key="i" class="w-full h-6" />
      </div>

      <p v-else-if="!processes.length" class="py-6 text-center text-sm text-muted-foreground">
        {{ $t('processes.empty') }}
      </p>

      <div v-else class="space-y-1.5">
        <div
          v-for="proc in processes"
          :key="proc.pid"
          class="relative flex items-center justify-between gap-2 text-xs font-mono py-1"
        >
          <!-- Bar scaled to the heaviest process, drawn behind the label so
               relative weight is readable at a glance. -->
          <div
            class="absolute inset-y-0 left-0 rounded bg-blue-500/10 pointer-events-none"
            :style="{ width: `${maxImpact ? (proc.impact / maxImpact) * 100 : 0}%` }"
          />
          <span class="relative truncate pl-1.5">{{ proc.name }}</span>
          <span class="relative shrink-0 pr-1.5 text-muted-foreground">
            {{ proc.impact.toFixed(1) }}
          </span>
        </div>
        <p class="pt-1 text-[10px] text-muted-foreground">
          {{ $t('processes.unit_note') }}
        </p>
      </div>
    </CardContent>
  </Card>
</template>
