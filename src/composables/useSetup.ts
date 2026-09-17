import { events } from '@/bindings'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useI18n } from 'vue-i18n'
import { useUpdater } from './useUpdater'

/** Give the app a moment to finish starting before hitting the network. */
const LAUNCH_CHECK_DELAY = 5000

export function useSetup() {
  const i18n = useI18n()
  const preferedLang = usePreferredLanguages()
  const preference = usePreference()
  const preferDark = usePreferredDark()
  const { checkForUpdate } = useUpdater()

  i18n.locale.value = preferedLang.value[0]

  const toggleDark = async () => {
    document.documentElement.classList.toggle('dark', preference.theme === 'system'
      ? preferDark.value
      : preference.theme === 'dark')
  }

  preference.$tauri.start().then(() => {
    toggleDark()
    i18n.locale.value = preference.language

    // Only the main window checks. useSetup runs in all three web views, so
    // keying off the window label avoids three concurrent requests, and the
    // shared updater state means the result still shows up in Settings.
    if (
      preference.autoCheckUpdates
      && getCurrentWebviewWindow().label === 'main'
    ) {
      // Silent: a launch check should surface a new version, not report that
      // there is nothing to do or that the network was unavailable.
      setTimeout(() => checkForUpdate(true), LAUNCH_CHECK_DELAY)
    }
  })

  watch([preferDark, () => preference.theme], toggleDark)

  events.preferenceEvent.listen(({ payload }) => {
    if ('theme' in payload) {
      toggleDark()
    }
    if ('language' in payload) {
      i18n.locale.value = payload.language
    }
  })

  // notify rust to get a instant update
  onMounted(() => {
    events.windowLoadedEvent.emit()
  })
}
