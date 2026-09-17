<script setup lang="tsx">
import type { Component, SetupContext } from 'vue'
import { Label } from '@/components/ui/label'
import {
  NumberField,
  NumberFieldContent,
  NumberFieldDecrement,
  NumberFieldIncrement,
  NumberFieldInput,
} from '@/components/ui/number-field'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Separator } from '@/components/ui/separator'
import { Switch } from '@/components/ui/switch'
import { open } from '@tauri-apps/plugin-shell'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { Activity, BadgeInfo, BatteryCharging, CircleDashed, ExternalLink, Eye, Gauge, Languages, Moon, Palette, RotateCw, Sun, SunMoon, Wallet } from 'lucide-vue-next'
import { storeToRefs } from 'pinia'
import { computed, h, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { version } from '../package.json'
import { events } from './bindings'
import { Skeleton } from './components/ui/skeleton'
import { usePreference } from './stores/preference'

const commitHash = __COMMIT_HASH__

const { t } = useI18n()

type UpdateState = 'idle' | 'checking' | 'available' | 'downloading' | 'none' | 'error'
const updateState = ref<UpdateState>('idle')
const updateVersion = ref('')

const updateLabel = computed(() => {
  switch (updateState.value) {
    case 'checking': return t('update.checking')
    case 'available': return t('update.install', { version: updateVersion.value })
    case 'downloading': return t('update.downloading')
    case 'none': return t('update.up_to_date')
    case 'error': return t('update.failed')
    default: return t('update.check')
  }
})

/**
 * Checks for an update, then installs it on a second click.
 *
 * The two phases share one button because the dialog the updater plugin shows
 * is disabled here — driving it from the settings window keeps the state
 * visible next to the version number instead of interrupting with a modal.
 */
async function checkForUpdate() {
  if (updateState.value === 'available') {
    await installUpdate()
    return
  }

  updateState.value = 'checking'
  try {
    const update = await check()
    if (update) {
      updateVersion.value = update.version
      updateState.value = 'available'
      pendingUpdate = update
    }
    else {
      updateState.value = 'none'
    }
  }
  catch (error) {
    console.error('[updater] check failed', error)
    updateState.value = 'error'
  }
}

let pendingUpdate: Awaited<ReturnType<typeof check>> = null

async function installUpdate() {
  if (!pendingUpdate) {
    return
  }
  updateState.value = 'downloading'
  try {
    await pendingUpdate.downloadAndInstall()
    // The new binary only takes effect after a restart.
    await relaunch()
  }
  catch (error) {
    console.error('[updater] install failed', error)
    updateState.value = 'error'
  }
}

useSetup()

const loading = ref(true)
const preference = usePreference()

preference.$tauri.start().then(async () => {
  const refs = storeToRefs(preference)

  for (const key in refs) {
    // TODO: fix types
    const ref = refs[key as keyof typeof refs]
    watch(ref, () => {
      events.preferenceEvent.emit({
        [key as keyof typeof refs]: ref.value,
      } as any)
    })
  }

  loading.value = false
})

interface SettingsItemProps {
  name: string
  description: string
  icon: Component
}

function SettingsItem(props: SettingsItemProps, { slots }: SetupContext) {
  return (
    <div class="flex items-center justify-between">
      <div class="flex gap-4">
        { h(props.icon, { class: 'size-5' }) }
        <div class="flex flex-col gap-2">
          <Label class="font-medium">{props.name}</Label>
          <span class="text-xs text-muted-foreground mr-4">
            {props.description}
          </span>
        </div>
      </div>
      {slots.default ? slots.default() : null}
    </div>
  )
}

interface SettingsSectionProps {
  title: string
  icon: Component
}

function SettingsSection(props: SettingsSectionProps) {
  return (
    <div>
      <h3 class="flex items-center gap-2 text-xl font-black">
        {/* {h(props.icon, { class: 'h-5 w-5' })} */}
        {props.title}
      </h3>
    </div>
  )
}
</script>

<template>
  <div data-tauri-drag-region class="h-6" />
  <div class="space-y-8 mt-6 px-8 bg-background overflow-y-scroll h-dvh">
    <!-- Appearance Section -->
    <SettingsSection :title="$t('settings.appearance')" :icon="Eye" />
    <div class="space-y-6">
      <SettingsItem
        :name="$t('settings.theme')"
        :description="$t('settings.theme_desc')"
        :icon="Palette"
      >
        <Select v-model="preference.theme">
          <SelectTrigger class="w-[130px]">
            <SelectValue placeholder="Select a theme" />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              <SelectItem value="system">
                <div class="flex items-center">
                  <SunMoon class="size-4 mr-3" />
                  {{ $t('settings.theme_system') }}
                </div>
              </SelectItem>
              <SelectItem class="flex" value="light">
                <div class="flex items-center">
                  <Sun class="size-4 mr-3" />
                  {{ $t('settings.theme_light') }}
                </div>
              </SelectItem>
              <SelectItem class="flex" value="dark">
                <div class="flex items-center">
                  <Moon class="size-4 mr-3" />
                  {{ $t('settings.theme_dark') }}
                </div>
              </SelectItem>
            </SelectGroup>
          </SelectContent>
        </Select>
      </SettingsItem>

      <SettingsItem
        :name="$t('settings.language')"
        :description="$t('settings.language_desc')"
        :icon="Languages"
      >
        <Select v-model="preference.language" default-value="en">
          <SelectTrigger class="w-[120px]">
            <SelectValue placeholder="Select a language" />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              <SelectItem value="en">
                English
              </SelectItem>
              <SelectItem value="zh-CN">
                简体中文
              </SelectItem>
            </SelectGroup>
          </SelectContent>
        </Select>
      </SettingsItem>

      <SettingsItem
        :name="$t('settings.animations')"
        :description="$t('settings.animations_desc')"
        :icon="CircleDashed"
      >
        <Skeleton v-if="loading" class="w-12 h-6" />
        <Switch
          v-else
          id="animations"
          v-model:checked="preference.animationsEnabled"
          class="data-[state=checked]:bg-blue-500"
        />
      </SettingsItem>
    </div>

    <Separator />

    <!-- Updates & Monitoring Section -->
    <SettingsSection
      :title="$t('settings.update_and_monitoring')"
      :icon="RotateCw"
    />
    <div class="space-y-6">
      <SettingsItem
        :name="$t('settings.update_frequency')"
        :description="$t('settings.update_frequency_desc')"
        :icon="Gauge"
      >
        <NumberField
          v-model="preference.updateInterval"
          :format-options="{
            useGrouping: false,
            style: 'unit',
            unit: 'millisecond',
            unitDisplay: 'short',
          }"
          locale="en-US"
          :min="500"
          :step="500"
          class="w-32"
        >
          <NumberFieldContent>
            <NumberFieldDecrement />
            <NumberFieldInput />
            <NumberFieldIncrement />
          </NumberFieldContent>
        </NumberField>
      </SettingsItem>

      <SettingsItem
        :name="$t('settings.background_monitoring')"
        :description="$t('settings.background_monitoring_desc')"
        :icon="Activity"
      >
        <Switch
          id="background-monitoring"
          class="data-[state=checked]:bg-blue-500"
          disabled
          checked
        />
      </SettingsItem>

      <SettingsItem
        :name="$t('settings.status_bar')"
        :description="$t('settings.status_bar_desc')"
        :icon="BadgeInfo"
      >
        <Select v-model="preference.statusBarItem" default-value="system">
          <SelectTrigger class="w-[150px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              <SelectItem value="system">
                {{ $t('settings.system_total') }}
              </SelectItem>
              <SelectItem value="screen">
                {{ $t('settings.screen_power') }}
              </SelectItem>
              <SelectItem value="heatpipe">
                {{ $t('settings.heatpipe_power') }}
              </SelectItem>
            </SelectGroup>
          </SelectContent>
        </Select>
      </SettingsItem>

      <SettingsItem
        :name="$t('settings.show_charging_power')"
        :description="$t('settings.show_charging_power_desc')"
        :icon="BatteryCharging"
      >
        <Switch v-model:checked="preference.statusBarShowCharging" class="data-[state=checked]:bg-blue-500" />
      </SettingsItem>
    </div>

    <!-- <Separator /> -->

    <!-- <section class="space-y-8">
      <SettingsSection title="Paired Devices" :icon="MobileIcon" />
      <div class="space-y-6">
        <SettingsItem
          name="iPhone 13 Pro"
          description="00000000-0000-0000-0000-000000000000"
          :icon="MobileIcon"
        >
          <Skeleton v-if="loading" class="w-12 h-6" />
          <Button
            variant="outline"
            size="sm"
            class="
          text-red-500 hover:text-red-600
          border-red-500/20 hover:border-red-500/50
          hover:bg-red-500/10
            "
          >
            Delete
          </Button>
        </SettingsItem>
      </div>
    </section> -->

    <Separator />

    <!-- About Section -->
    <SettingsSection :title="$t('settings.about')" :icon="Wallet" />
    <div class="grid grid-cols-2 gap-4">
      <div>
        <div class="text-sm font-medium text-muted-foreground">
          {{ $t('settings.version') }}
        </div>
        <div class="text-sm flex items-center gap-2">
          {{ version }}
          <button
            class="text-xs text-muted-foreground underline cursor-pointer disabled:opacity-50 disabled:cursor-default"
            :disabled="updateState === 'checking' || updateState === 'downloading'"
            @click="checkForUpdate"
          >
            {{ updateLabel }}
          </button>
        </div>
      </div>
      <div>
        <div class="text-sm font-medium text-muted-foreground">
          {{ $t('settings.build') }}
        </div>
        <div class="text-sm flex items-center">
          {{ commitHash.slice(0, 7) }}
          <a
            class="ml-2 mr-1 text-xs text-muted-foreground underline flex items-center gap-1 cursor-pointer"
            @click="open(`https://github.com/swsususu/Trickle/commit/${commitHash}`)"
          >
            View on GitHub
            <ExternalLink class="size-3 text-muted-foreground" />
          </a>
        </div>
      </div>
      <div>
        <div class="text-sm font-medium text-muted-foreground">
          {{ $t('settings.license') }}
        </div>
        <div class="text-sm">
          MIT License
        </div>
      </div>
      <div>
        <div class="text-sm font-medium text-muted-foreground">
          {{ $t('settings.author') }}
        </div>
        <div class="text-sm flex items-center flex-wrap gap-x-1">
          <a
            class="underline cursor-pointer"
            @click="open('https://github.com/swsususu')"
          >swsususu</a>
          <span class="text-muted-foreground">·</span>
          <!-- Upstream author: powerflow, which Trickle is based on. -->
          <a
            class="underline cursor-pointer"
            @click="open('https://github.com/lzt1008')"
          >Samuel Lyon</a>
        </div>
      </div>
    </div>

    <div class="h-8" />
  </div>
</template>
