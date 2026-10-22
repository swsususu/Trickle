import type { App } from 'vue'
import messages from '@intlify/unplugin-vue-i18n/messages'
import { MotionPlugin } from '@vueuse/motion'
import { createPlugin as createTauriPiniaPlugin } from 'tauri-plugin-pinia'
import { createI18n } from 'vue-i18n'
import '../assets/index.css'

const i18n = createI18n({
  locale: 'en',
  fallbackLocale: 'en',
  messages,
})

// WebKit freezes a hidden document's animation timeline, so a CSS transition
// started there never finishes. Every tick restarts the transitions on the
// power bar and load meters, and each replaced one leaks memory inside
// WebContent: the status bar panel's process (always "hidden", see usePower)
// grew by ~120 MB per 1000 ticks, reaching GBs after a day (issue #4).
// Switching transitions off while hidden keeps it flat.
function pauseTransitionsWhileHidden() {
  const sync = () => document.documentElement.classList
    .toggle('no-transitions', document.visibilityState === 'hidden')
  sync()
  document.addEventListener('visibilitychange', sync)
}

export function setup(entry: Component, fn?: (app: App<Element>) => void) {
  pauseTransitionsWhileHidden()

  const app = createApp(entry)
    .use(MotionPlugin)
    .use(
      createPinia()
        .use(createTauriPiniaPlugin()),
    )
    .use(i18n)

  if (fn) {
    fn(app)
  }

  app.mount('#app')
}
