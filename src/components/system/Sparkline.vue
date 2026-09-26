<script setup lang="ts">
/** Minimal SVG sparkline. Cheaper than a chart instance per card. */
const props = withDefaults(defineProps<{
  values: number[]
  /** Fixed ceiling, e.g. 100 for percentages. Defaults to the series max. */
  max?: number
  color: string
  /** Optional second series drawn on the same scale, without fill. */
  second?: number[]
  secondColor?: string
  capacity?: number
}>(), { capacity: 60 })

const H = 36
const W = 100

function path(values: number[], max: number) {
  if (values.length < 2)
    return ''
  const step = W / (props.capacity - 1)
  const offset = (props.capacity - values.length) * step
  return values
    .map((v, i) => `${i ? 'L' : 'M'}${(offset + i * step).toFixed(2)},${(H - (v / max) * H * 0.9).toFixed(2)}`)
    .join('')
}

const scale = computed(() => props.max ?? Math.max(1, ...props.values, ...(props.second ?? [])))
const line = computed(() => path(props.values, scale.value))
const area = computed(() => {
  if (!line.value)
    return ''
  const startX = line.value.slice(1).split(',')[0]
  return `${line.value}L${W},${H}L${startX},${H}Z`
})
const secondLine = computed(() => (props.second ? path(props.second, scale.value) : ''))
</script>

<template>
  <svg
    class="w-full h-9 block"
    :viewBox="`0 0 ${W} ${H}`"
    preserveAspectRatio="none"
    aria-hidden="true"
  >
    <path :d="area" :fill="color" opacity="0.15" />
    <path
      :d="line"
      fill="none"
      :stroke="color"
      stroke-width="1.5"
      vector-effect="non-scaling-stroke"
    />
    <path
      v-if="secondLine"
      :d="secondLine"
      fill="none"
      :stroke="secondColor"
      stroke-width="1.5"
      vector-effect="non-scaling-stroke"
    />
  </svg>
</template>
