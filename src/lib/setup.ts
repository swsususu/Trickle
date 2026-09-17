import type { App } from 'vue'
import messages from '@intlify/unplugin-vue-i18n/messages'
import { attachConsole } from '@tauri-apps/plugin-log'
import { MotionPlugin } from '@vueuse/motion'
import { createPlugin as createTauriPiniaPlugin } from 'tauri-plugin-pinia'
import { createI18n } from 'vue-i18n'
import '../assets/index.css'

const i18n = createI18n({
  locale: 'en',
  fallbackLocale: 'en',
  messages,
})

export function setup(entry: Component, fn?: (app: App<Element>) => void) {
  // Forward the web view's console into the Rust log file. Without this,
  // console.error from the frontend goes only to the dev tools, so anything
  // logged there is invisible in a release build — which is how an updater
  // problem could look like silence in Trickle.log.
  attachConsole().catch(() => {
    // A missing log bridge must not stop the app from starting.
  })

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
