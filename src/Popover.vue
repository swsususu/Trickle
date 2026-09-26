<script setup lang="ts">
import { commands } from '@/bindings'

useSetup()

// The NSPopover has a fixed content size, so grow or shrink it to fit when
// the system stats row appears or disappears.
const content = useTemplateRef<HTMLElement>('content')
let lastHeight = 0
useResizeObserver(content, (entries) => {
  const height = Math.ceil(entries[0].contentRect.height)
  if (height > 0 && Math.abs(height - lastHeight) >= 1) {
    lastHeight = height
    commands.setPopoverHeight(height)
  }
})
</script>

<template>
  <div ref="content">
    <PowerStatusPopover>
      <PowerStatus />
    </PowerStatusPopover>
    <SystemMiniStats />
  </div>
</template>

<style>
body {
  background: transparent;
}
</style>
