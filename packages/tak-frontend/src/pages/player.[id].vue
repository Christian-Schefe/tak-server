<script setup lang="ts">
import { useAccount } from '@/api/auth';
import { usePlayerInfo } from '@/api/player';
import {
  useProfile,
  useProfilePictureUrl,
  useUpdateProfile,
  useUploadProfilePicture,
} from '@/api/profile';
import FlagIcon from '@/components/FlagIcon.vue';
import PlayerStats from '@/components/PlayerStats.vue';
import RatingHistory from '@/components/RatingHistory.vue';
import { countryOptions } from '@/utils/flags';
import { Button, Dialog, Form, Select } from '@tak-ui-lib/components';
import { computed, ref } from 'vue';
import { LuPen } from 'vue-icons-plus/lu';
import { useRoute } from 'vue-router';

const route = useRoute('/player.[id]');

const { data: playerInfo } = usePlayerInfo(() => route.params.id);
const { data: profile } = useProfile(() => playerInfo.value?.accountId);
const avatarUrl = useProfilePictureUrl(() => playerInfo.value?.accountId);
const { data: account } = useAccount();
const canEditProfile = computed(() => {
  return (
    account.value !== undefined &&
    account.value.playerId === route.params.id &&
    !account.value.isGuest
  );
});

const editDialogVisible = ref(false);

const { mutate: uploadProfilePicture, isPending: isUploadingProfilePicture } =
  useUploadProfilePicture();

const { mutate: updateProfile, isPending: isUpdatingProfile } = useUpdateProfile();

function onUpload(uploadEvent: FileUploadSelectEvent) {
  if (!playerInfo.value) {
    return;
  }
  const file = uploadEvent.files[0] as File | undefined;
  if (!file) {
    return;
  }
  uploadProfilePicture({ accountId: playerInfo.value.accountId, file });
}

function onUpdateProfile(event: FormSubmitEvent) {
  if (!playerInfo.value) {
    return;
  }
  const country = event.values.country as string | null;
  updateProfile({ accountId: playerInfo.value.accountId, country });
}
</script>
<template>
  <div class="w-full mx-auto max-w-6xl p-4 flex flex-col gap-4">
    <div class="flex flex-row gap-4">
      <div
        class="w-32 h-full aspect-square rounded-lg p-0 overflow-hidden flex items-center justify-center"
      >
        <img
          v-if="avatarUrl !== undefined"
          :src="avatarUrl"
          alt="Player Avatar"
          class="w-full h-full pointer-events-none"
        />
      </div>
      <div v-if="playerInfo" class="flex flex-col grow">
        <div class="font-bold text-2xl flex items-center gap-2">
          <h1>{{ playerInfo.displayName }}</h1>
          <FlagIcon :country="profile?.country ?? undefined" />
        </div>
        <p class="text-muted-color mb-4">@{{ playerInfo.username }}</p>
      </div>
      <div v-if="canEditProfile">
        <Button severity="secondary" class="aspect-square" @click="editDialogVisible = true">
          <template #icon><LuPen class="w-5 h-5" /></template>
        </Button>
      </div>
    </div>
    <PlayerStats :player-id="route.params.id" />
    <h1>Rating History</h1>
    <RatingHistory :player-id="route.params.id" />
  </div>
  <Dialog v-model:visible="editDialogVisible" header="Your Profile">
    <div class="w-full flex flex-col items-center gap-4">
      <div
        class="w-64 h-64 rounded-lg border border-surface overflow-hidden flex items-center justify-center"
      >
        <img
          v-if="avatarUrl !== undefined && !isUploadingProfilePicture"
          :src="avatarUrl"
          alt="Profile Picture"
          class="w-full h-full pointer-events-none"
        />
      </div>
      <FileUpload
        :multiple="false"
        accept="image/*"
        :max-file-size="1000000"
        custom-upload
        mode="basic"
        :auto="true"
        @select="onUpload"
      >
      </FileUpload>
      <p class="text-muted-color text-center">
        Recommended size: 256x256 pixels<br />Maximum file size: 1MB
      </p>
      <Form
        :initial-values="{ country: profile?.country || null }"
        class="w-full max-w-100 flex flex-col"
        @submit="onUpdateProfile"
      >
        <Select model-value="" name="country" :options="countryOptions" label="Country"></Select>
        <Button type="submit" label="Update Profile" :disabled="isUpdatingProfile" />
      </Form>
    </div>
  </Dialog>
</template>
