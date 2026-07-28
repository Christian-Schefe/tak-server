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
import { countryArray } from '@/utils/flags';
import { Form, type FormSubmitEvent } from '@primevue/forms';
import Button from 'primevue/button';
import Card from 'primevue/card';
import Dialog from 'primevue/dialog';
import FileUpload, { type FileUploadSelectEvent } from 'primevue/fileupload';
import IftaLabel from 'primevue/iftalabel';
import Select from 'primevue/select';
import Skeleton from 'primevue/skeleton';
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
    <Card>
      <template #content>
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
            <Skeleton v-else class="h-full! w-full!" />
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
      </template>
    </Card>
    <PlayerStats :player-id="route.params.id" />
    <Card>
      <template #title>Rating History</template>
      <template #content>
        <RatingHistory :player-id="route.params.id" />
      </template>
    </Card>
  </div>
  <Dialog
    v-model:visible="editDialogVisible"
    modal
    dismissable-mask
    header="Your Profile"
    :draggable="false"
    :style="{ width: '50vw' }"
    :breakpoints="{ '1199px': '75vw', '575px': '90vw' }"
  >
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
        <Skeleton v-else class="h-full! w-full!" />
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
        <IftaLabel>
          <Select
            name="country"
            :options="countryArray"
            option-label="name"
            option-value="code"
            filter
            fluid
          ></Select>
          <label>Country</label>
        </IftaLabel>
        <Button
          type="submit"
          label="Update Profile"
          class="mt-4"
          :disabled="isUpdatingProfile"
          fluid
        />
      </Form>
    </div>
  </Dialog>
</template>
