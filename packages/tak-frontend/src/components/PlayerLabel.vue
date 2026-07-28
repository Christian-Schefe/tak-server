<script setup lang="ts">
import { useIsAccountOnline } from '@/api/account';
import { useAccountOrPlayerInfo } from '@/api/player';
import { useProfile, useProfilePictureUrl } from '@/api/profile';
import FlagIcon from '@/components/FlagIcon.vue';
import OverlayBadge from 'primevue/overlaybadge';
import Skeleton from 'primevue/skeleton';

const props = withDefaults(
  defineProps<{
    pid: string;
    type: 'account' | 'player';
    showProfilePicture?: boolean;
    showRating?: boolean;
    showFlag?: boolean;
  }>(),
  {
    showProfilePicture: true,
    showRating: true,
    showFlag: true,
  },
);

const {
  data: playerInfo,
  isError,
  isLoading,
} = useAccountOrPlayerInfo(
  () => props.pid,
  () => props.type,
);
const { data: profile } = useProfile(() => playerInfo.value?.accountId);
const avatarSrc = useProfilePictureUrl(() => playerInfo.value?.accountId);

const isOnline = useIsAccountOnline(() => playerInfo.value?.accountId);
</script>
<template>
  <RouterLink
    :to="`/player/${playerInfo?.playerId}`"
    :draggable="false"
    class="flex gap-2 items-center justify-start hover:underline"
  >
    <OverlayBadge v-if="showProfilePicture && isOnline === true" severity="success">
      <img :src="avatarSrc" alt="Profile Picture" class="w-8 h-8 rounded-sm pointer-events-none" />
    </OverlayBadge>
    <img
      v-else-if="showProfilePicture"
      :src="avatarSrc"
      alt="Profile Picture"
      class="w-8 h-8 rounded-sm pointer-events-none"
    />

    <Skeleton v-if="isLoading" border-radius="4px" class="h-8! grow" />
    <span v-else class="text-left text-ellipsis overflow-hidden text-nowrap">
      {{ playerInfo?.displayName ?? (isError ? 'Unknown Player' : '') }}
      <span
        v-if="
          showRating &&
          playerInfo?.participationRating !== undefined &&
          playerInfo.participationRating !== null
        "
        class="text-muted-color text-sm font-mono"
      >
        {{ ' ' }}({{ playerInfo.participationRating.toFixed(0) }})
      </span>
    </span>
    <FlagIcon v-if="showFlag" :country="profile?.country ?? undefined" />
  </RouterLink>
</template>
