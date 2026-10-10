<script lang="ts" setup>
import { useOnlineUsers } from '@/api/account';
import Page from '@/components/Page.vue';
import PlayerLabel from '@/components/PlayerLabel.vue';
import ChatPanel from '@/components/side-panel/ChatPanel.vue';
import { Card } from '@tak-ui-lib/components';
import { computed } from 'vue';

const { data: onlineAccounts } = useOnlineUsers();
const onlineuserIds = computed(() => {
  const userIds = onlineAccounts.value ? [...onlineAccounts.value] : [];
  userIds.sort((a, b) => a.localeCompare(b));
  return userIds;
});
</script>
<template>
  <Page>
    <Card class="w-full">
      <h2>Online Players</h2>
      <div class="flex flex-col gap-2">
        <PlayerLabel v-for="userId in onlineuserIds" :key="userId" :user-id="userId" />
      </div>
    </Card>
    <ChatPanel :conversation="{ type: 'global' }" />
  </Page>
</template>
