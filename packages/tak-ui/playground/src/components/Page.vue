<script setup lang="ts"></script>
<template>
  <div class="mx-auto w-full max-w-[1664px] flex">
    <div class="w-0 grow"></div>
    <div class="w-full max-w-4xl p-4 flex flex-col gap-4">
      <slot />
    </div>
    <div class="w-0 grow-2"></div>
  </div>
</template>
