<template>
  <div class="flex flex-col gap-4 min-w-min pb-2 px-4 pt-2">
    <div class="flex gap-6">
      <PowerStatus />
      <PowerFlow />
    </div>
    <PowerUsageChart />
    <TechnicalDetail />
    <div class="grid gap-4 grid-cols-1 lg:grid-cols-2">
      <BatteryHealthChart />
      <ProcessEnergyList />
    </div>
    <AdapterDetail />
  </div>
</template>
