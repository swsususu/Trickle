import type { Update } from '@tauri-apps/plugin-updater'
import { error as logError, info as logInfo, warn as logWarn } from '@tauri-apps/plugin-log'
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification'
import { relaunch } from '@tauri-apps/plugin-process'
import { check } from '@tauri-apps/plugin-updater'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

export type UpdateState =
  | 'idle'
  | 'checking'
  | 'available'
  | 'downloading'
  | 'none'
  | 'error'

/** Shared so the settings window and the launch check agree on one state. */
const state = ref<UpdateState>('idle')
const availableVersion = ref('')
let pending: Update | null = null

/**
 * Tells the user a new version exists.
 *
 * The launch check runs while the app is usually just a menu bar icon, so the
 * state change alone would go unseen; a system notification is the only way to
 * surface it without stealing focus.
 */
async function notifyAvailable(version: string) {
  try {
    let granted = await isPermissionGranted()
    if (!granted) {
      granted = (await requestPermission()) === 'granted'
    }
    if (granted) {
      sendNotification({
        title: 'Trickle',
        body: `Version ${version} is available. Open Settings to install it.`,
      })
      logInfo('[updater] notification sent')
    }
    else {
      logWarn('[updater] notification permission denied')
    }
  }
  catch (error) {
    // A missing notification must not make the update itself look failed.
    logError(`[updater] notification failed: ${error}`)
  }
}

/**
 * Checks for a new release, and installs it on a second call.
 *
 * `silent` suppresses the "up to date" and error outcomes, for the check that
 * runs at launch: telling someone their app is current when they did not ask
 * is noise, and a failed check usually means the network is down.
 */
async function checkForUpdate(silent = false): Promise<void> {
  if (state.value === 'checking' || state.value === 'downloading') {
    return
  }

  state.value = 'checking'
  try {
    const update = await check()
    if (update) {
      pending = update
      availableVersion.value = update.version
      state.value = 'available'
      logInfo(`[updater] ${update.version} available`)
      if (silent) {
        await notifyAvailable(update.version)
      }
    }
    else {
      // Logged so a launch check that found nothing is distinguishable from
      // one that never ran.
      logInfo('[updater] no update available')
      state.value = silent ? 'idle' : 'none'
    }
  }
  catch (error) {
    logError(`[updater] check failed: ${error}`)
    state.value = silent ? 'idle' : 'error'
  }
}

/** Downloads and installs the pending update, then restarts. */
async function installUpdate(): Promise<void> {
  if (!pending) {
    return
  }
  state.value = 'downloading'
  try {
    await pending.downloadAndInstall()
    // The replaced binary only takes effect after a restart.
    await relaunch()
  }
  catch (error) {
    logError(`[updater] install failed: ${error}`)
    state.value = 'error'
  }
}

export function useUpdater() {
  const { t } = useI18n()

  const label = computed(() => {
    switch (state.value) {
      case 'checking': return t('update.checking')
      case 'available': return t('update.install', { version: availableVersion.value })
      case 'downloading': return t('update.downloading')
      case 'none': return t('update.up_to_date')
      case 'error': return t('update.failed')
      default: return t('update.check')
    }
  })

  const busy = computed(() =>
    state.value === 'checking' || state.value === 'downloading',
  )

  /** One control for both phases: check, then install what was found. */
  async function activate() {
    if (state.value === 'available') {
      await installUpdate()
    }
    else {
      await checkForUpdate()
    }
  }

  return {
    state,
    availableVersion,
    label,
    busy,
    activate,
    checkForUpdate,
    installUpdate,
  }
}
