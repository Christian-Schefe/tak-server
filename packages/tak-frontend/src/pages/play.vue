<script setup lang="ts">
import { useAccount } from '@/api/auth';
import { useAcceptSeek, useCreateSeek, useDeleteSeek, useSeeks, type SeekInfo } from '@/api/seek';
import CreateSeekModal from '@/components/CreateSeekModal.vue';
import SeekSummary from '@/components/SeekSummary.vue';
import Button from 'primevue/button';
import { computed, ref } from 'vue';

const { data: seeks } = useSeeks();

const { data: account } = useAccount();

type SeekEntry = {
  type: 'own' | 'other';
  seek: SeekInfo;
};

const seekData = computed(() => {
  const ownSeeks: SeekEntry[] = [];
  const otherSeeks: SeekEntry[] = [];
  seeks.value?.forEach((seek) => {
    if (seek.creatorId === account.value?.playerId) {
      ownSeeks.push({ type: 'own', seek });
    } else {
      otherSeeks.push({ type: 'other', seek });
    }
  });
  return { seeks: ownSeeks.concat(otherSeeks), ownSeeks, otherSeeks };
});

const { mutate: createSeek } = useCreateSeek();
const { mutate: acceptSeek } = useAcceptSeek();
const { mutate: deleteSeek } = useDeleteSeek();

function onAcceptSeek(seekId: string) {
  console.log(`Accepting seek ${seekId}`);
  acceptSeek(seekId);
}

function onDeleteSeek(seekId: string) {
  console.log(`Deleting seek ${seekId}`);
  deleteSeek(seekId);
}

const createSeekDialogVisible = ref(false);
</script>
<template>
  <div class="w-full mx-auto max-w-4xl p-2 pt-4 flex flex-col gap-6">
    <div class="flex flex-col gap-2">
      <div class="flex items-center">
        <h1 class="text-2xl font-semibold">Your Seeks</h1>
        <div class="grow"></div>
        <Button label="Create Seek" size="small" @click="createSeekDialogVisible = true" />
      </div>
      <SeekSummary
        v-for="seek in seekData.ownSeeks"
        :key="seek.seek.id"
        :seek="seek.seek"
        :action="'delete'"
        @click="onDeleteSeek(seek.seek.id)"
      ></SeekSummary>
      <p v-if="!seekData.ownSeeks.length" class="text-muted-color">
        You have no active seeks. Click "Create Seek" to create a new one.
      </p>
    </div>
    <div class="flex flex-col gap-2">
      <h1 class="text-2xl font-semibold">Seeks</h1>
      <SeekSummary
        v-for="seek in seekData.otherSeeks"
        :key="seek.seek.id"
        :seek="seek.seek"
        :action="'accept'"
        @click="onAcceptSeek(seek.seek.id)"
      ></SeekSummary>
      <p v-if="!seekData.otherSeeks.length" class="text-muted-color">No seeks available.</p>
    </div>
  </div>
  <CreateSeekModal v-model="createSeekDialogVisible" @create="createSeek" />
</template>
