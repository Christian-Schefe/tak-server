<script lang="ts" setup>
import { useOnlineAccounts } from '@/api/account';
import Page from '@/components/Page.vue';
import PlayerLabel from '@/components/PlayerLabel.vue';
import ChatPanel from '@/components/side-panel/ChatPanel.vue';
import { Card } from '@tak-ui-lib/components';
import { computed } from 'vue';

const { data: onlineAccounts } = useOnlineAccounts();
const onlineAccountIds = computed(() => {
  const accountIds = onlineAccounts.value ? [...onlineAccounts.value] : [];
  accountIds.sort((a, b) => a.localeCompare(b));
  return accountIds;
});
</script>
<template>
  <Page>
    <Card class="w-full">
      <h2>Online Players</h2>
      <div class="flex flex-col gap-2">
        <PlayerLabel
          v-for="accountId in onlineAccountIds"
          :key="accountId"
          :pid="accountId"
          type="account"
        />
      </div>
    </Card>
    <Card class="w-full">
      <h2>Global Chat</h2>
      <div class="h-100">
        <ChatPanel :conversation="{ type: 'global' }" />
      </div>
    </Card>
  </Page>
</template>
