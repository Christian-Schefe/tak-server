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
import Page from '@/components/Page.vue';
import PlayerStats from '@/components/PlayerStats.vue';
import RatingHistory from '@/components/RatingHistory.vue';
import { countryOptions } from '@/utils/flags';
import { zodFormValidator } from '@/utils/forms';
import { Button, useFormContext, Dialog, Form, Select } from '@tak-ui-lib/components';
import { computed, ref, useTemplateRef } from 'vue';
import { LuPen, LuPlus } from 'vue-icons-plus/lu';
import { useRoute } from 'vue-router';
import z from 'zod';

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
const profilePictureDialogVisible = ref(false);

const { mutate: uploadProfilePicture, isPending: isUploadingProfilePicture } =
  useUploadProfilePicture();

const { mutate: updateProfile, isPending: isUpdatingProfile } = useUpdateProfile();

function onUpload(uploadEvent: Event) {
  if (!playerInfo.value) {
    return;
  }
  const target = uploadEvent.target as HTMLInputElement;
  const file = target.files?.[0];
  if (!file) {
    return;
  }
  uploadProfilePicture({ accountId: playerInfo.value.accountId, file });
}

function onUpdateProfile(event: { country: string | null }) {
  if (!playerInfo.value) {
    return;
  }
  updateProfile({ accountId: playerInfo.value.accountId, country: event.country });
}

const formCtx = useFormContext(() => ({
  country: profile.value?.country ?? null,
}));

const validator = zodFormValidator(
  z.object({
    country: z.string().nullable(),
  }),
);

const profilePictureInput = useTemplateRef<HTMLInputElement>('profilePictureInput');
function openProfilePictureSelect() {
  profilePictureInput.value?.click();
}

function onClickProfilePicture() {
  if (!canEditProfile.value) {
    return;
  }
  profilePictureDialogVisible.value = true;
}
</script>
<template>
  <Page>
    <div class="flex flex-row gap-4">
      <div
        class="w-32 h-full aspect-square rounded-lg p-0 overflow-hidden flex items-center justify-center cursor-pointer"
        @click="onClickProfilePicture"
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
        <p class="mb-4">@{{ playerInfo.username }}</p>
      </div>
      <div v-if="canEditProfile">
        <Button severity="secondary" variant="text" icon-only @click="editDialogVisible = true">
          <LuPen />
        </Button>
      </div>
    </div>
    <PlayerStats :player-id="route.params.id" />
    <h1>Rating History</h1>
    <RatingHistory :player-id="route.params.id" />
  </Page>
  <Dialog v-model:visible="editDialogVisible" header="Your Profile">
    <Form v-model="formCtx" :validator="validator" @submit="onUpdateProfile">
      <div class="w-full flex flex-col gap-4">
        <Select model-value="" name="country" :options="countryOptions" label="Country"></Select>
        <div class="flex justify-end gap-2">
          <Button
            severity="secondary"
            variant="text"
            label="Cancel"
            @click="editDialogVisible = false"
          />
          <Button
            type="submit"
            variant="text"
            label="Update Profile"
            :disabled="isUpdatingProfile"
          />
        </div>
      </div>
    </Form>
  </Dialog>
  <Dialog v-model:visible="profilePictureDialogVisible" header="Profile Picture">
    <div class="w-full flex flex-col items-center gap-4">
      <div class="w-64 h-64 rounded-lg overflow-hidden flex items-center justify-center">
        <img
          v-if="avatarUrl !== undefined && !isUploadingProfilePicture"
          :src="avatarUrl"
          alt="Profile Picture"
          class="w-full h-full pointer-events-none"
        />
      </div>
      <Button label="Change Picture" @click="openProfilePictureSelect">
        <template #icon><LuPlus /></template>
      </Button>
      <input
        ref="profilePictureInput"
        class="hidden"
        type="file"
        accept="image/*"
        :disabled="isUploadingProfilePicture"
        @change="onUpload($event)"
      />
    </div>
  </Dialog>
</template>
