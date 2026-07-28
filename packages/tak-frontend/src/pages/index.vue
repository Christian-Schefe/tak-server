<script setup lang="ts">
import Card from 'primevue/card';
import { Fa6Discord } from 'vue-icons-plus/fa6';
import { LuGraduationCap } from 'vue-icons-plus/lu';
</script>

<template>
  <div class="w-full mx-auto max-w-6xl p-4 grid grid-cols-1 md:grid-cols-2 gap-4">
    <Card class="w-full parallax-bg">
      <template #content>
        <a
          class="w-full h-40 flex items-center justify-center cursor-pointer hover:underline text-white"
          href="https://ustak.org/play-beautiful-game-tak/"
        >
          <div class="flex items-center gap-2 text-3xl font-semibold">
            <LuGraduationCap class="w-10 h-10" />
            <p>Learn to play Tak</p>
          </div>
        </a>
      </template>
    </Card>
    <Card class="w-full discord-bg">
      <template #content>
        <a
          class="w-full h-40 flex items-center justify-center cursor-pointer hover:underline text-white"
          href="https://discord.gg/2xEt42X"
        >
          <div class="flex items-center gap-2 text-3xl font-semibold">
            <Fa6Discord class="w-10 h-10" />
            <p>Join our community</p>
          </div>
        </a>
      </template>
    </Card>
  </div>
</template>
<style lang="css" scoped>
.parallax-bg {
  background:
    linear-gradient(to right, rgba(12, 94, 153, 0.3), rgb(12, 94, 153, 0.3)), url('/parallax.jpg');
  background-position: center, center;
  background-repeat: no-repeat, no-repeat;
  background-size: cover, cover;
}
.discord-bg {
  background-color: rgb(88, 101, 242);
}
</style>
