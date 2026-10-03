<script setup lang="ts">
import { useAccount } from '@/api/auth';
import { useAcceptSeek, useCreateSeek, useDeleteSeek, useSeeks, type SeekInfo } from '@/api/seek';
import CreateSeekModal from '@/components/CreateSeekModal.vue';
import Page from '@/components/Page.vue';
import SeekSummary from '@/components/SeekSummary.vue';
import { Button } from '@tak-ui-lib/components';
import { computed, ref } from 'vue';
import { LuPlus } from 'vue-icons-plus/lu';

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
  <Page>
    <h1 class="text-2xl font-semibold">Your Seeks</h1>
    <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
      <SeekSummary
        v-for="seek in seekData.ownSeeks"
        :key="seek.seek.id"
        :seek="seek.seek"
        :action="'delete'"
        @click="onDeleteSeek(seek.seek.id)"
      ></SeekSummary>
    </div>
    <div class="flex justify-center">
      <Button label="Create Seek" @click="createSeekDialogVisible = true"
        ><template #icon><LuPlus /></template
      ></Button>
    </div>
    <h1 class="text-2xl font-semibold">Seeks</h1>
    <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
      <SeekSummary
        v-for="seek in seekData.otherSeeks"
        :key="seek.seek.id"
        :seek="seek.seek"
        :action="'accept'"
        @click="onAcceptSeek(seek.seek.id)"
      ></SeekSummary>
    </div>
    <p v-if="!seekData.otherSeeks.length">No seeks available.</p>
  </Page>
  <CreateSeekModal v-model="createSeekDialogVisible" @create="createSeek" />
</template>
